use flate2::read::GzDecoder;
use maxminddb::{Reader, path};
pub use sabicheiro_core::{AddressScope, classify_address};
use sabicheiro_core::{
    AsnInfo, GeoDatabaseStatus, GeoDatasetStatus, GeoLocation, GeoPrecision, PlacementKind,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, VecDeque},
    fs,
    io::{self, Read},
    net::IpAddr,
    path::{Path, PathBuf},
    time::Duration,
};
use thiserror::Error;

const DEFAULT_CACHE_CAPACITY: usize = 8_192;
const MAX_COMPRESSED_DATABASE_BYTES: u64 = 256 * 1024 * 1024;
const MAX_DATABASE_BYTES: u64 = 512 * 1024 * 1024;
const DNS_POSITIVE_TTL_MS: i64 = 3_600_000;
const DNS_NEGATIVE_TTL_MS: i64 = 300_000;

#[derive(Debug, Error)]
pub enum GeoError {
    #[error("MMDB error: {0}")]
    Database(#[from] maxminddb::MaxMindDbError),
    #[error("download error: {0}")]
    Download(#[from] reqwest::Error),
    #[error("filesystem error: {0}")]
    Io(#[from] io::Error),
    #[error("downloaded database exceeds the 256 MiB safety limit")]
    DownloadTooLarge,
    #[error("downloaded database contains no usable MMDB data")]
    InvalidDatabase,
    #[error("MMDB edition '{actual}' does not match expected {expected} data")]
    InvalidEdition {
        expected: &'static str,
        actual: String,
    },
    #[error("imported database exceeds the 512 MiB safety limit")]
    ImportTooLarge,
    #[error("decompressed database exceeds the 512 MiB safety limit")]
    DatabaseTooLarge,
    #[error("no validated rollback database is available for {0}")]
    NoBackup(&'static str),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoLookup {
    pub scope: AddressScope,
    pub location: Option<GeoLocation>,
    pub asn: Option<AsnInfo>,
    pub city_generation: u64,
    pub asn_generation: u64,
}

impl GeoLookup {
    fn scoped(scope: AddressScope, city_generation: u64, asn_generation: u64) -> Self {
        Self {
            scope,
            location: None,
            asn: None,
            city_generation,
            asn_generation,
        }
    }
}

pub struct LocalGeoResolver {
    city: Option<Reader<Vec<u8>>>,
    asn: Option<Reader<Vec<u8>>>,
    cache: BoundedCache<GeoCacheKey, GeoLookup>,
    city_path: Option<PathBuf>,
    asn_path: Option<PathBuf>,
    city_identity: Option<DatasetIdentity>,
    asn_identity: Option<DatasetIdentity>,
    city_generation: u64,
    asn_generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DatasetIdentity {
    edition: String,
    build_epoch_ms: i64,
    sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GeoCacheKey {
    address: IpAddr,
    city_generation: u64,
    asn_generation: u64,
}

impl LocalGeoResolver {
    pub fn empty() -> Self {
        Self {
            city: None,
            asn: None,
            cache: BoundedCache::new(DEFAULT_CACHE_CAPACITY),
            city_path: None,
            asn_path: None,
            city_identity: None,
            asn_identity: None,
            city_generation: 0,
            asn_generation: 0,
        }
    }

    pub fn open(
        city_path: Option<impl AsRef<Path>>,
        asn_path: Option<impl AsRef<Path>>,
    ) -> Result<Self, GeoError> {
        let city_path = city_path.map(|path| path.as_ref().to_path_buf());
        let asn_path = asn_path.map(|path| path.as_ref().to_path_buf());
        let city = city_path
            .as_deref()
            .filter(|path| path.exists())
            .map(Reader::open_readfile)
            .transpose()?;
        let asn = asn_path
            .as_deref()
            .filter(|path| path.exists())
            .map(Reader::open_readfile)
            .transpose()?;
        validate_reader_kind(city.as_ref(), GeoDatabaseKind::City)?;
        validate_reader_kind(asn.as_ref(), GeoDatabaseKind::Asn)?;
        let city_identity = city
            .as_ref()
            .zip(city_path.as_deref())
            .map(|(reader, path)| dataset_identity(reader, path))
            .transpose()?;
        let asn_identity = asn
            .as_ref()
            .zip(asn_path.as_deref())
            .map(|(reader, path)| dataset_identity(reader, path))
            .transpose()?;
        Ok(Self {
            city,
            asn,
            cache: BoundedCache::new(DEFAULT_CACHE_CAPACITY),
            city_path,
            asn_path,
            city_generation: u64::from(city_identity.is_some()),
            asn_generation: u64::from(asn_identity.is_some()),
            city_identity,
            asn_identity,
        })
    }

    pub fn reload(
        &mut self,
        city_path: impl AsRef<Path>,
        asn_path: impl AsRef<Path>,
    ) -> Result<(), GeoError> {
        let city = Reader::open_readfile(city_path.as_ref())?;
        let asn = Reader::open_readfile(asn_path.as_ref())?;
        validate_reader_kind(Some(&city), GeoDatabaseKind::City)?;
        validate_reader_kind(Some(&asn), GeoDatabaseKind::Asn)?;
        let city_identity = dataset_identity(&city, city_path.as_ref())?;
        let asn_identity = dataset_identity(&asn, asn_path.as_ref())?;
        self.city = Some(city);
        self.asn = Some(asn);
        self.city_path = Some(city_path.as_ref().to_path_buf());
        self.asn_path = Some(asn_path.as_ref().to_path_buf());
        self.city_identity = Some(city_identity);
        self.asn_identity = Some(asn_identity);
        self.city_generation = self.city_generation.saturating_add(1).max(1);
        self.asn_generation = self.asn_generation.saturating_add(1).max(1);
        self.cache.clear();
        Ok(())
    }

    pub fn reload_kind(
        &mut self,
        kind: GeoDatabaseKind,
        path: impl AsRef<Path>,
    ) -> Result<(), GeoError> {
        let reader = Reader::open_readfile(path.as_ref())?;
        validate_reader_kind(Some(&reader), kind)?;
        let identity = dataset_identity(&reader, path.as_ref())?;
        match kind {
            GeoDatabaseKind::City => {
                self.city = Some(reader);
                self.city_path = Some(path.as_ref().to_path_buf());
                self.city_identity = Some(identity);
                self.city_generation = self.city_generation.saturating_add(1).max(1);
            }
            GeoDatabaseKind::Asn => {
                self.asn = Some(reader);
                self.asn_path = Some(path.as_ref().to_path_buf());
                self.asn_identity = Some(identity);
                self.asn_generation = self.asn_generation.saturating_add(1).max(1);
            }
        }
        self.cache.clear();
        Ok(())
    }

    pub fn lookup(&mut self, address: IpAddr) -> Result<GeoLookup, GeoError> {
        let cache_key = GeoCacheKey {
            address,
            city_generation: self.city_generation,
            asn_generation: self.asn_generation,
        };
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        let scope = classify_address(address);
        if !scope.is_global() {
            let result = GeoLookup::scoped(scope, self.city_generation, self.asn_generation);
            self.cache.insert(cache_key, result.clone());
            return Ok(result);
        }

        let location = self
            .city
            .as_ref()
            .map(|reader| {
                decode_location(
                    reader,
                    address,
                    self.city_identity.as_ref(),
                    self.city_generation,
                    self.asn_generation,
                )
            })
            .transpose()?
            .flatten();
        let asn = self
            .asn
            .as_ref()
            .map(|reader| {
                decode_asn(
                    reader,
                    address,
                    self.asn_identity.as_ref(),
                    self.asn_generation,
                )
            })
            .transpose()?
            .flatten();
        let result = GeoLookup {
            scope,
            location,
            asn,
            city_generation: self.city_generation,
            asn_generation: self.asn_generation,
        };
        self.cache.insert(cache_key, result.clone());
        Ok(result)
    }

    pub fn status(&self) -> GeoDatabaseStatus {
        let city = dataset_status(
            GeoDatabaseKind::City,
            self.city.as_ref(),
            self.city_path.as_deref(),
            self.city_identity.as_ref(),
            self.city_generation,
        );
        let asn = dataset_status(
            GeoDatabaseKind::Asn,
            self.asn.as_ref(),
            self.asn_path.as_deref(),
            self.asn_identity.as_ref(),
            self.asn_generation,
        );
        GeoDatabaseStatus {
            installed: city.installed,
            dataset: city.dataset.clone(),
            version: city.version.clone(),
            last_update_ms: city.last_update_ms,
            next_recommended_update_ms: city.next_recommended_update_ms,
            path: city.path.clone(),
            edition: city.edition.clone(),
            sha256: city.sha256.clone(),
            attribution: "IP geolocation by DB-IP.com, licensed under CC BY 4.0".into(),
            city,
            asn,
            last_operation: None,
        }
    }

    pub const fn generations(&self) -> (u64, u64) {
        (self.city_generation, self.asn_generation)
    }
}

fn dataset_status(
    kind: GeoDatabaseKind,
    reader: Option<&Reader<Vec<u8>>>,
    path: Option<&Path>,
    identity: Option<&DatasetIdentity>,
    generation: u64,
) -> GeoDatasetStatus {
    let build_epoch_ms = identity.map(|value| value.build_epoch_ms);
    let last_update_ms = path
        .and_then(|path| fs::metadata(path).ok())
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64);
    GeoDatasetStatus {
        kind: kind.label().to_ascii_lowercase(),
        installed: reader.is_some(),
        dataset: identity
            .map(|value| value.edition.clone())
            .unwrap_or_else(|| format!("{} MMDB (not installed)", kind.label())),
        version: build_epoch_ms.and_then(format_epoch_month),
        build_epoch_ms,
        last_update_ms,
        next_recommended_update_ms: build_epoch_ms.map(|value| value + 32 * 86_400_000),
        path: path.map(|path| path.to_string_lossy().into_owned()),
        edition: identity.map(|value| value.edition.clone()),
        sha256: identity.map(|value| value.sha256.clone()),
        generation,
        backup_available: path.is_some_and(|path| path.with_extension("mmdb.bak").is_file()),
        attribution: "IP geolocation by DB-IP.com, licensed under CC BY 4.0".into(),
    }
}

fn validate_reader_kind<S: AsRef<[u8]>>(
    reader: Option<&Reader<S>>,
    kind: GeoDatabaseKind,
) -> Result<(), GeoError> {
    let Some(reader) = reader else {
        return Ok(());
    };
    let metadata = reader.metadata();
    if metadata.node_count == 0 || metadata.database_type.trim().is_empty() {
        return Err(GeoError::InvalidDatabase);
    }
    if !metadata
        .database_type
        .to_ascii_lowercase()
        .contains(&kind.label().to_ascii_lowercase())
    {
        return Err(GeoError::InvalidEdition {
            expected: kind.label(),
            actual: metadata.database_type.clone(),
        });
    }
    Ok(())
}

fn dataset_identity(reader: &Reader<Vec<u8>>, path: &Path) -> Result<DatasetIdentity, GeoError> {
    let build_epoch_ms = reader.metadata().build_epoch.min(i64::MAX as u64 / 1_000) as i64 * 1_000;
    Ok(DatasetIdentity {
        edition: reader.metadata().database_type.clone(),
        build_epoch_ms,
        sha256: file_sha256(path)?,
    })
}

fn decode_location(
    reader: &Reader<Vec<u8>>,
    address: IpAddr,
    identity: Option<&DatasetIdentity>,
    city_generation: u64,
    asn_generation: u64,
) -> Result<Option<GeoLocation>, maxminddb::MaxMindDbError> {
    let lookup = reader.lookup(address)?;
    if !lookup.has_data() {
        return Ok(None);
    }
    let country_code: Option<String> = lookup
        .decode_path(&path!["country", "iso_code"])?
        .filter(|value: &String| {
            value.len() == 2 && value.bytes().all(|byte| byte.is_ascii_alphabetic())
        })
        .map(|value| value.to_ascii_uppercase());
    let country_name: Option<String> = lookup
        .decode_path(&path!["country", "names", "en"])?
        .and_then(clean_text);
    let latitude: Option<f64> = lookup.decode_path(&path!["location", "latitude"])?;
    let longitude: Option<f64> = lookup.decode_path(&path!["location", "longitude"])?;
    let coordinates = latitude.zip(longitude).filter(|(latitude, longitude)| {
        latitude.is_finite()
            && longitude.is_finite()
            && (-90.0..=90.0).contains(latitude)
            && (-180.0..=180.0).contains(longitude)
    });
    let region = lookup
        .decode_path(&path!["subdivisions", 0, "names", "en"])?
        .and_then(clean_text);
    let city = lookup
        .decode_path(&path!["city", "names", "en"])?
        .and_then(clean_text);
    let accuracy_radius_km = lookup.decode_path(&path!["location", "accuracy_radius"])?;
    if country_code.is_none()
        && country_name.is_none()
        && region.is_none()
        && city.is_none()
        && coordinates.is_none()
    {
        return Ok(None);
    }
    let precision = if coordinates.is_some() {
        GeoPrecision::Coordinates
    } else if city.is_some() {
        GeoPrecision::City
    } else if region.is_some() {
        GeoPrecision::Region
    } else if country_code.is_some() || country_name.is_some() {
        GeoPrecision::Country
    } else {
        GeoPrecision::Unknown
    };
    let matched_prefix = lookup.network().ok().map(|network| network.prefix());
    let (latitude, longitude) = coordinates.unzip();
    Ok(Some(GeoLocation {
        country_code,
        country_name,
        region,
        city,
        latitude,
        longitude,
        source: "DB-IP Lite local MMDB".into(),
        precision,
        placement: if latitude.is_some() && longitude.is_some() {
            PlacementKind::DatabaseCoordinate
        } else {
            PlacementKind::NotPlotted
        },
        accuracy_radius_km,
        dataset_kind: Some("city".into()),
        dataset_edition: identity.map(|value| value.edition.clone()),
        dataset_build_epoch_ms: identity.map(|value| value.build_epoch_ms),
        dataset_sha256: identity.map(|value| value.sha256.clone()),
        matched_prefix,
        city_generation,
        asn_generation,
    }))
}

fn decode_asn(
    reader: &Reader<Vec<u8>>,
    address: IpAddr,
    identity: Option<&DatasetIdentity>,
    generation: u64,
) -> Result<Option<AsnInfo>, maxminddb::MaxMindDbError> {
    let lookup = reader.lookup(address)?;
    if !lookup.has_data() {
        return Ok(None);
    }
    let number: Option<u32> = lookup.decode_path(&path!["autonomous_system_number"])?;
    let organization: Option<String> = lookup
        .decode_path(&path!["autonomous_system_organization"])?
        .and_then(clean_text);
    Ok(number.map(|number| AsnInfo {
        number,
        organization,
        dataset_edition: identity.map(|value| value.edition.clone()),
        dataset_build_epoch_ms: identity.map(|value| value.build_epoch_ms),
        dataset_sha256: identity.map(|value| value.sha256.clone()),
        matched_prefix: lookup.network().ok().map(|network| network.prefix()),
        generation,
    }))
}

fn clean_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.chars().take(256).collect())
    }
}

#[derive(Debug, Clone)]
struct DnsCacheEntry {
    hostname: Option<String>,
    expires_at_ms: i64,
}

pub struct DnsResolver {
    entries: BoundedCache<IpAddr, DnsCacheEntry>,
}

impl Default for DnsResolver {
    fn default() -> Self {
        Self {
            entries: BoundedCache::new(4_096),
        }
    }
}

impl DnsResolver {
    pub fn resolve(&mut self, address: IpAddr, now_ms: i64) -> Option<String> {
        if classify_address(address) == AddressScope::Unspecified {
            return None;
        }
        if let Some(entry) = self.entries.get(&address) {
            if entry.expires_at_ms > now_ms {
                return entry.hostname.clone();
            }
        }
        let hostname = dns_lookup::lookup_addr(&address)
            .ok()
            .map(|value| value.trim_end_matches('.').to_owned())
            .filter(|value| !value.is_empty() && value != &address.to_string());
        let ttl = if hostname.is_some() {
            DNS_POSITIVE_TTL_MS
        } else {
            DNS_NEGATIVE_TTL_MS
        };
        self.entries.insert(
            address,
            DnsCacheEntry {
                hostname: hostname.clone(),
                expires_at_ms: now_ms.saturating_add(ttl),
            },
        );
        hostname
    }
}

pub struct UpdatedDatabases {
    pub city_path: PathBuf,
    pub asn_path: PathBuf,
    pub dataset_month: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeoDatabaseKind {
    City,
    Asn,
}

impl GeoDatabaseKind {
    const fn label(self) -> &'static str {
        match self {
            Self::City => "City",
            Self::Asn => "ASN",
        }
    }

    const fn destination_name(self) -> &'static str {
        match self {
            Self::City => "dbip-city-lite.mmdb",
            Self::Asn => "dbip-asn-lite.mmdb",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedDatabase {
    pub kind: GeoDatabaseKind,
    pub path: PathBuf,
    pub edition: String,
    pub sha256: String,
}

pub fn import_mmdb(
    source: impl AsRef<Path>,
    data_directory: impl AsRef<Path>,
    kind: GeoDatabaseKind,
) -> Result<ImportedDatabase, GeoError> {
    let source = source.as_ref();
    let metadata = fs::metadata(source)?;
    if !metadata.is_file() || metadata.len() > 512 * 1024 * 1024 {
        return Err(GeoError::ImportTooLarge);
    }
    let database = fs::read(source)?;
    let reader = Reader::from_source(database.as_slice())?;
    if reader.metadata().node_count == 0 {
        return Err(GeoError::InvalidDatabase);
    }
    let edition = reader.metadata().database_type.clone();
    if !edition
        .to_ascii_lowercase()
        .contains(&kind.label().to_ascii_lowercase())
    {
        return Err(GeoError::InvalidEdition {
            expected: kind.label(),
            actual: edition,
        });
    }
    fs::create_dir_all(data_directory.as_ref())?;
    let destination = data_directory.as_ref().join(kind.destination_name());
    install_database(&database, &destination)?;
    Ok(ImportedDatabase {
        kind,
        path: destination,
        edition,
        sha256: format!("{:x}", Sha256::digest(&database)),
    })
}

pub fn update_dbip_lite(data_directory: impl AsRef<Path>) -> Result<UpdatedDatabases, GeoError> {
    let data_directory = data_directory.as_ref();
    fs::create_dir_all(data_directory)?;
    let month = chrono::Utc::now().format("%Y-%m").to_string();
    let city_url = format!("https://download.db-ip.com/free/dbip-city-lite-{month}.mmdb.gz");
    let asn_url = format!("https://download.db-ip.com/free/dbip-asn-lite-{month}.mmdb.gz");
    let city_path = data_directory.join("dbip-city-lite.mmdb");
    let asn_path = data_directory.join("dbip-asn-lite.mmdb");
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .user_agent(concat!(
            "Sabicheiro/",
            env!("CARGO_PKG_VERSION"),
            " (+local offline GeoIP updater)"
        ))
        .build()?;

    download_and_install(&client, &city_url, &city_path, GeoDatabaseKind::City)?;
    if let Err(error) = download_and_install(&client, &asn_url, &asn_path, GeoDatabaseKind::Asn) {
        // City data remains useful. Do not remove a previously valid ASN database.
        if !asn_path.exists() {
            return Err(error);
        }
    }
    Ok(UpdatedDatabases {
        city_path,
        asn_path,
        dataset_month: month,
    })
}

fn download_and_install(
    client: &reqwest::blocking::Client,
    url: &str,
    destination: &Path,
    kind: GeoDatabaseKind,
) -> Result<(), GeoError> {
    let mut response = client.get(url).send()?.error_for_status()?;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_COMPRESSED_DATABASE_BYTES)
    {
        return Err(GeoError::DownloadTooLarge);
    }
    let mut compressed = Vec::new();
    response
        .by_ref()
        .take(MAX_COMPRESSED_DATABASE_BYTES + 1)
        .read_to_end(&mut compressed)?;
    if compressed.len() as u64 > MAX_COMPRESSED_DATABASE_BYTES {
        return Err(GeoError::DownloadTooLarge);
    }
    let mut decoder = GzDecoder::new(compressed.as_slice());
    let mut database = Vec::new();
    decoder
        .by_ref()
        .take(MAX_DATABASE_BYTES + 1)
        .read_to_end(&mut database)?;
    if database.len() as u64 > MAX_DATABASE_BYTES {
        return Err(GeoError::DatabaseTooLarge);
    }
    let reader = Reader::from_source(database.as_slice())?;
    if reader.metadata().node_count == 0 {
        return Err(GeoError::InvalidDatabase);
    }
    validate_reader_kind(Some(&reader), kind)?;

    install_database(&database, destination)
}

fn install_database(database: &[u8], destination: &Path) -> Result<(), GeoError> {
    let pending = destination.with_extension("mmdb.new");
    let backup = destination.with_extension("mmdb.bak");
    fs::write(&pending, database)?;
    if destination.exists() {
        if backup.exists() {
            fs::remove_file(&backup)?;
        }
        fs::rename(destination, &backup)?;
    }
    if let Err(error) = fs::rename(&pending, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(error.into());
    }
    Ok(())
}

pub fn rollback_mmdb(
    data_directory: impl AsRef<Path>,
    kind: GeoDatabaseKind,
) -> Result<ImportedDatabase, GeoError> {
    let destination = data_directory.as_ref().join(kind.destination_name());
    let backup = destination.with_extension("mmdb.bak");
    if !backup.is_file() {
        return Err(GeoError::NoBackup(kind.label()));
    }
    let backup_data = fs::read(&backup)?;
    let reader = Reader::from_source(backup_data.as_slice())?;
    validate_reader_kind(Some(&reader), kind)?;
    let edition = reader.metadata().database_type.clone();
    let rollback = destination.with_extension("mmdb.rollback");
    if rollback.exists() {
        fs::remove_file(&rollback)?;
    }
    if destination.exists() {
        fs::rename(&destination, &rollback)?;
    }
    if let Err(error) = fs::rename(&backup, &destination) {
        if rollback.exists() {
            let _ = fs::rename(&rollback, &destination);
        }
        return Err(error.into());
    }
    if rollback.exists() {
        fs::rename(&rollback, &backup)?;
    }
    Ok(ImportedDatabase {
        kind,
        path: destination,
        edition,
        sha256: format!("{:x}", Sha256::digest(&backup_data)),
    })
}

pub fn remove_mmdb(
    data_directory: impl AsRef<Path>,
    kind: GeoDatabaseKind,
) -> Result<bool, GeoError> {
    let destination = data_directory.as_ref().join(kind.destination_name());
    let candidates = [
        destination.clone(),
        destination.with_extension("mmdb.bak"),
        destination.with_extension("mmdb.new"),
        destination.with_extension("mmdb.rollback"),
    ];
    let mut removed = false;
    for candidate in candidates {
        if candidate.is_file() {
            fs::remove_file(candidate)?;
            removed = true;
        }
    }
    Ok(removed)
}

fn file_sha256(path: &Path) -> io::Result<String> {
    let data = fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(data)))
}

fn format_epoch_month(epoch_ms: i64) -> Option<String> {
    chrono::DateTime::from_timestamp_millis(epoch_ms).map(|date| date.format("%Y-%m").to_string())
}

struct BoundedCache<K, V> {
    values: HashMap<K, V>,
    order: VecDeque<K>,
    capacity: usize,
}

impl<K, V> BoundedCache<K, V>
where
    K: Clone + Eq + std::hash::Hash,
{
    fn new(capacity: usize) -> Self {
        Self {
            values: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
            capacity: capacity.max(1),
        }
    }

    fn get(&self, key: &K) -> Option<&V> {
        self.values.get(key)
    }

    fn insert(&mut self, key: K, value: V) {
        if !self.values.contains_key(&key) {
            self.order.push_back(key.clone());
        }
        self.values.insert(key, value);
        while self.values.len() > self.capacity {
            if let Some(expired) = self.order.pop_front() {
                self.values.remove(&expired);
            }
        }
    }

    fn clear(&mut self) {
        self.values.clear();
        self.order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mmdb_writer::{Writer, ipnet::IpNet};
    use serde::Serialize;
    use std::{collections::BTreeMap, time::SystemTime};

    #[derive(Serialize)]
    struct FixtureNames {
        names: BTreeMap<String, String>,
    }

    #[derive(Serialize)]
    struct FixtureCountry {
        iso_code: Option<String>,
        names: BTreeMap<String, String>,
    }

    #[derive(Serialize)]
    struct FixtureLocation {
        latitude: Option<f64>,
        longitude: Option<f64>,
        accuracy_radius: Option<u16>,
    }

    #[derive(Serialize)]
    struct FixtureCity {
        country: Option<FixtureCountry>,
        subdivisions: Vec<FixtureNames>,
        city: Option<FixtureNames>,
        location: Option<FixtureLocation>,
    }

    #[derive(Serialize)]
    struct FixtureAsn {
        autonomous_system_number: u32,
        autonomous_system_organization: Option<String>,
    }

    fn names(value: &str) -> BTreeMap<String, String> {
        [("en".into(), value.into())].into()
    }

    fn fixture_city(
        country: Option<(&str, &str)>,
        region: Option<&str>,
        city: Option<&str>,
        coordinates: Option<(f64, f64)>,
    ) -> FixtureCity {
        FixtureCity {
            country: country.map(|(code, name)| FixtureCountry {
                iso_code: Some(code.into()),
                names: names(name),
            }),
            subdivisions: region
                .map(|value| {
                    vec![FixtureNames {
                        names: names(value),
                    }]
                })
                .unwrap_or_default(),
            city: city.map(|value| FixtureNames {
                names: names(value),
            }),
            location: coordinates.map(|(latitude, longitude)| FixtureLocation {
                latitude: Some(latitude),
                longitude: Some(longitude),
                accuracy_radius: Some(12),
            }),
        }
    }

    fn writer(database_type: &str) -> Writer {
        Writer::builder(database_type)
            .build_epoch(SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000))
            .build()
    }

    fn write_city_fixture(path: &Path, city_name: &str) {
        let mut writer = writer("Sabicheiro-City-Test");
        writer
            .insert(
                "8.8.0.0/16".parse::<IpNet>().expect("base network"),
                &fixture_city(Some(("AA", "Exampleland")), Some("North"), None, None),
            )
            .expect("base city record");
        writer
            .insert(
                "8.8.8.0/24".parse::<IpNet>().expect("specific network"),
                &fixture_city(
                    Some(("AA", "Exampleland")),
                    Some("Central"),
                    Some(city_name),
                    Some((12.5, -44.25)),
                ),
            )
            .expect("specific city record");
        writer
            .insert(
                "9.9.9.0/24"
                    .parse::<IpNet>()
                    .expect("invalid coordinate network"),
                &fixture_city(
                    Some(("BB", "Partialia")),
                    None,
                    Some("Invalid Point"),
                    Some((120.0, f64::INFINITY)),
                ),
            )
            .expect("invalid coordinate record");
        writer
            .insert(
                "11.0.0.0/24".parse::<IpNet>().expect("partial network"),
                &fixture_city(Some(("CC", "Country Only")), None, None, None),
            )
            .expect("partial record");
        writer
            .insert(
                "2606:4700::/32".parse::<IpNet>().expect("IPv6 network"),
                &fixture_city(
                    Some(("DD", "Version Six")),
                    Some("IPv6 Region"),
                    Some("IPv6 City"),
                    Some((-12.0, 80.0)),
                ),
            )
            .expect("IPv6 record");
        fs::write(path, writer.to_bytes().expect("city MMDB bytes")).expect("city fixture");
    }

    fn write_asn_fixture(path: &Path) {
        let mut writer = writer("Sabicheiro-ASN-Test");
        writer
            .insert(
                "8.8.0.0/16".parse::<IpNet>().expect("ASN base network"),
                &FixtureAsn {
                    autonomous_system_number: 64_500,
                    autonomous_system_organization: Some("Example Network".into()),
                },
            )
            .expect("ASN base record");
        writer
            .insert(
                "8.8.8.0/24".parse::<IpNet>().expect("ASN specific network"),
                &FixtureAsn {
                    autonomous_system_number: 64_501,
                    autonomous_system_organization: None,
                },
            )
            .expect("ASN specific record");
        writer
            .insert(
                "2606:4700::/32".parse::<IpNet>().expect("ASN IPv6 network"),
                &FixtureAsn {
                    autonomous_system_number: 64_502,
                    autonomous_system_organization: Some("Example IPv6 Network".into()),
                },
            )
            .expect("ASN IPv6 record");
        fs::write(path, writer.to_bytes().expect("ASN MMDB bytes")).expect("ASN fixture");
    }

    #[test]
    fn address_scope_never_geolocates_private_ranges() {
        assert_eq!(
            classify_address("192.168.1.1".parse().expect("IP")),
            AddressScope::Private
        );
        assert_eq!(
            classify_address("fd00::1".parse().expect("IP")),
            AddressScope::Private
        );
        assert_eq!(
            classify_address("127.0.0.1".parse().expect("IP")),
            AddressScope::Loopback
        );
        assert_eq!(
            classify_address("1.1.1.1".parse().expect("IP")),
            AddressScope::Global
        );
    }

    #[test]
    fn geo_cache_is_bounded_and_returns_scope_without_database() {
        let mut resolver = LocalGeoResolver::empty();
        let lookup = resolver
            .lookup("10.0.0.1".parse().expect("IP"))
            .expect("lookup");
        assert_eq!(lookup.scope, AddressScope::Private);
        assert!(lookup.location.is_none());
    }

    #[test]
    fn bounded_cache_evicts_oldest_entry() {
        let mut cache = BoundedCache::new(2);
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");
        assert!(cache.get(&1).is_none());
        assert_eq!(cache.get(&3), Some(&"three"));
    }

    #[test]
    fn dns_cache_returns_positive_and_negative_entries_without_lookup() {
        let positive: IpAddr = "203.0.113.10".parse().expect("IP");
        let negative: IpAddr = "203.0.113.11".parse().expect("IP");
        let mut resolver = DnsResolver::default();
        resolver.entries.insert(
            positive,
            DnsCacheEntry {
                hostname: Some("cached.example".into()),
                expires_at_ms: 10_000,
            },
        );
        resolver.entries.insert(
            negative,
            DnsCacheEntry {
                hostname: None,
                expires_at_ms: 10_000,
            },
        );

        assert_eq!(
            resolver.resolve(positive, 9_999).as_deref(),
            Some("cached.example")
        );
        assert_eq!(resolver.resolve(negative, 9_999), None);
    }

    #[test]
    fn synthetic_mmdb_fixture_covers_prefixes_families_partial_and_invalid_data() {
        let directory = tempfile::tempdir().expect("fixture directory");
        let city = directory.path().join("city.mmdb");
        let asn = directory.path().join("asn.mmdb");
        write_city_fixture(&city, "Specific City");
        write_asn_fixture(&asn);
        let mut resolver = LocalGeoResolver::open(Some(&city), Some(&asn)).expect("resolver");

        let exact = resolver
            .lookup("8.8.8.8".parse().expect("specific IP"))
            .expect("specific lookup");
        let location = exact.location.expect("specific location");
        assert_eq!(location.city.as_deref(), Some("Specific City"));
        assert_eq!(location.matched_prefix, Some(24));
        assert_eq!(location.precision, GeoPrecision::Coordinates);
        assert_eq!(location.coordinates(), Some((12.5, -44.25)));
        let exact_asn = exact.asn.expect("specific ASN");
        assert_eq!(exact_asn.number, 64_501);
        assert!(exact_asn.organization.is_none());
        assert_eq!(exact_asn.matched_prefix, Some(24));

        let longest_prefix = resolver
            .lookup("8.8.9.9".parse().expect("base IP"))
            .expect("base lookup");
        assert_eq!(
            longest_prefix
                .location
                .as_ref()
                .and_then(|value| value.country_name.as_deref()),
            Some("Exampleland")
        );
        assert!(
            longest_prefix
                .location
                .as_ref()
                .and_then(|value| value.matched_prefix)
                .is_some_and(|prefix| prefix >= 16)
        );
        assert_eq!(longest_prefix.asn.expect("base ASN").number, 64_500);

        let invalid = resolver
            .lookup("9.9.9.9".parse().expect("invalid coordinate IP"))
            .expect("invalid coordinate lookup")
            .location
            .expect("partial invalid location");
        assert_eq!(invalid.precision, GeoPrecision::City);
        assert_eq!(invalid.placement, PlacementKind::NotPlotted);
        assert!(invalid.coordinates().is_none());

        let partial = resolver
            .lookup("11.0.0.1".parse().expect("partial IP"))
            .expect("partial lookup")
            .location
            .expect("partial location");
        assert_eq!(partial.precision, GeoPrecision::Country);
        assert!(partial.coordinates().is_none());

        let ipv6 = resolver
            .lookup("2606:4700::1234".parse().expect("IPv6 IP"))
            .expect("IPv6 lookup");
        assert_eq!(
            ipv6.location.and_then(|value| value.city),
            Some("IPv6 City".into())
        );
        assert_eq!(ipv6.asn.expect("IPv6 ASN").number, 64_502);

        let no_match = resolver
            .lookup("12.0.0.1".parse().expect("unmatched IP"))
            .expect("unmatched lookup");
        assert!(no_match.location.is_none());
        assert!(no_match.asn.is_none());
    }

    #[test]
    fn reload_invalidates_cache_and_advances_only_the_changed_generation() {
        let directory = tempfile::tempdir().expect("fixture directory");
        let city_one = directory.path().join("city-one.mmdb");
        let city_two = directory.path().join("city-two.mmdb");
        let asn = directory.path().join("asn.mmdb");
        write_city_fixture(&city_one, "First City");
        write_city_fixture(&city_two, "Second City");
        write_asn_fixture(&asn);
        let mut resolver = LocalGeoResolver::open(Some(&city_one), Some(&asn)).expect("resolver");
        let first = resolver
            .lookup("8.8.8.8".parse().expect("IP"))
            .expect("first lookup");
        assert_eq!(
            first.location.and_then(|value| value.city),
            Some("First City".into())
        );
        assert_eq!(first.city_generation, 1);
        assert_eq!(first.asn_generation, 1);

        resolver
            .reload_kind(GeoDatabaseKind::City, &city_two)
            .expect("city reload");
        let second = resolver
            .lookup("8.8.8.8".parse().expect("IP"))
            .expect("second lookup");
        assert_eq!(
            second.location.and_then(|value| value.city),
            Some("Second City".into())
        );
        assert_eq!(second.city_generation, 2);
        assert_eq!(second.asn_generation, 1);
    }

    #[test]
    fn import_rejects_wrong_or_truncated_editions_and_rollback_preserves_active_data() {
        let directory = tempfile::tempdir().expect("fixture directory");
        let city_one = directory.path().join("city-one.mmdb");
        let city_two = directory.path().join("city-two.mmdb");
        let asn = directory.path().join("asn.mmdb");
        let truncated = directory.path().join("truncated.mmdb");
        let installed = directory.path().join("installed");
        write_city_fixture(&city_one, "First City");
        write_city_fixture(&city_two, "Second City");
        write_asn_fixture(&asn);
        fs::write(&truncated, b"not an MMDB").expect("truncated fixture");

        assert!(matches!(
            import_mmdb(&asn, &installed, GeoDatabaseKind::City),
            Err(GeoError::InvalidEdition { .. })
        ));
        assert!(import_mmdb(&truncated, &installed, GeoDatabaseKind::City).is_err());
        let first =
            import_mmdb(&city_one, &installed, GeoDatabaseKind::City).expect("first import");
        let first_hash = first.sha256;
        import_mmdb(&city_two, &installed, GeoDatabaseKind::City).expect("second import");
        let restored = rollback_mmdb(&installed, GeoDatabaseKind::City).expect("rollback");
        assert_eq!(restored.sha256, first_hash);
        let mut resolver =
            LocalGeoResolver::open(Some(&restored.path), Option::<&Path>::None).expect("resolver");
        assert_eq!(
            resolver
                .lookup("8.8.8.8".parse().expect("IP"))
                .expect("restored lookup")
                .location
                .and_then(|value| value.city),
            Some("First City".into())
        );
        assert!(remove_mmdb(&installed, GeoDatabaseKind::City).expect("remove"));
        assert!(!restored.path.exists());
    }
}
