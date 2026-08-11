use flate2::read::GzDecoder;
use maxminddb::{Reader, path};
use sabicheiro_core::{AsnInfo, GeoDatabaseStatus, GeoLocation};
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AddressScope {
    Public,
    Loopback,
    LocalNetwork,
    LinkLocal,
    Multicast,
    Unspecified,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeoLookup {
    pub scope: AddressScope,
    pub location: Option<GeoLocation>,
    pub asn: Option<AsnInfo>,
}

impl GeoLookup {
    fn scoped(scope: AddressScope) -> Self {
        Self {
            scope,
            location: None,
            asn: None,
        }
    }
}

pub struct LocalGeoResolver {
    city: Option<Reader<Vec<u8>>>,
    asn: Option<Reader<Vec<u8>>>,
    cache: BoundedCache<IpAddr, GeoLookup>,
    city_path: Option<PathBuf>,
    asn_path: Option<PathBuf>,
}

impl LocalGeoResolver {
    pub fn empty() -> Self {
        Self {
            city: None,
            asn: None,
            cache: BoundedCache::new(DEFAULT_CACHE_CAPACITY),
            city_path: None,
            asn_path: None,
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
        Ok(Self {
            city,
            asn,
            cache: BoundedCache::new(DEFAULT_CACHE_CAPACITY),
            city_path,
            asn_path,
        })
    }

    pub fn reload(
        &mut self,
        city_path: impl AsRef<Path>,
        asn_path: impl AsRef<Path>,
    ) -> Result<(), GeoError> {
        let city = Reader::open_readfile(city_path.as_ref())?;
        let asn = Reader::open_readfile(asn_path.as_ref())?;
        self.city = Some(city);
        self.asn = Some(asn);
        self.city_path = Some(city_path.as_ref().to_path_buf());
        self.asn_path = Some(asn_path.as_ref().to_path_buf());
        self.cache.clear();
        Ok(())
    }

    pub fn lookup(&mut self, address: IpAddr) -> Result<GeoLookup, GeoError> {
        if let Some(cached) = self.cache.get(&address) {
            return Ok(cached.clone());
        }
        let scope = classify_address(address);
        if scope != AddressScope::Public {
            let result = GeoLookup::scoped(scope);
            self.cache.insert(address, result.clone());
            return Ok(result);
        }

        let location = self
            .city
            .as_ref()
            .map(|reader| decode_location(reader, address))
            .transpose()?
            .flatten();
        let asn = self
            .asn
            .as_ref()
            .map(|reader| decode_asn(reader, address))
            .transpose()?
            .flatten();
        let result = GeoLookup {
            scope,
            location,
            asn,
        };
        self.cache.insert(address, result.clone());
        Ok(result)
    }

    pub fn status(&self) -> GeoDatabaseStatus {
        let metadata = self.city.as_ref().map(|reader| reader.metadata());
        let build_epoch = metadata.map(|value| value.build_epoch as i64 * 1_000);
        GeoDatabaseStatus {
            installed: self.city.is_some(),
            dataset: metadata
                .map(|value| value.database_type.clone())
                .unwrap_or_else(|| "DB-IP City Lite (not installed)".into()),
            version: build_epoch.and_then(format_epoch_month),
            last_update_ms: self
                .city_path
                .as_ref()
                .and_then(|path| fs::metadata(path).ok())
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64),
            next_recommended_update_ms: build_epoch.map(|value| value + 32 * 86_400_000),
            path: self
                .city_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            edition: metadata.map(|value| value.database_type.clone()),
            sha256: self
                .city_path
                .as_deref()
                .filter(|path| path.is_file())
                .and_then(|path| file_sha256(path).ok()),
            attribution: "IP geolocation by DB-IP.com, licensed under CC BY 4.0".into(),
        }
    }
}

fn decode_location(
    reader: &Reader<Vec<u8>>,
    address: IpAddr,
) -> Result<Option<GeoLocation>, maxminddb::MaxMindDbError> {
    let lookup = reader.lookup(address)?;
    let country_code: Option<String> = lookup.decode_path(&path!["country", "iso_code"])?;
    let country_name: Option<String> = lookup
        .decode_path(&path!["country", "names", "en"])?
        .or_else(|| country_code.clone());
    let latitude: Option<f64> = lookup.decode_path(&path!["location", "latitude"])?;
    let longitude: Option<f64> = lookup.decode_path(&path!["location", "longitude"])?;
    let (Some(country_code), Some(country_name), Some(latitude), Some(longitude)) =
        (country_code, country_name, latitude, longitude)
    else {
        return Ok(None);
    };
    if !latitude.is_finite()
        || !longitude.is_finite()
        || !(-90.0..=90.0).contains(&latitude)
        || !(-180.0..=180.0).contains(&longitude)
    {
        return Ok(None);
    }
    let region = lookup.decode_path(&path!["subdivisions", 0, "names", "en"])?;
    let city = lookup.decode_path(&path!["city", "names", "en"])?;
    Ok(Some(GeoLocation {
        country_code,
        country_name,
        region,
        city,
        latitude,
        longitude,
        source: "DB-IP Lite local MMDB".into(),
    }))
}

fn decode_asn(
    reader: &Reader<Vec<u8>>,
    address: IpAddr,
) -> Result<Option<AsnInfo>, maxminddb::MaxMindDbError> {
    let lookup = reader.lookup(address)?;
    let number: Option<u32> = lookup.decode_path(&path!["autonomous_system_number"])?;
    let organization: Option<String> =
        lookup.decode_path(&path!["autonomous_system_organization"])?;
    Ok(number.map(|number| AsnInfo {
        number,
        organization: organization.unwrap_or_else(|| "Unknown organization".into()),
    }))
}

pub fn classify_address(address: IpAddr) -> AddressScope {
    if address.is_unspecified() {
        return AddressScope::Unspecified;
    }
    if address.is_loopback() {
        return AddressScope::Loopback;
    }
    if address.is_multicast() {
        return AddressScope::Multicast;
    }
    match address {
        IpAddr::V4(address) if address.is_private() => AddressScope::LocalNetwork,
        IpAddr::V4(address) if address.is_link_local() => AddressScope::LinkLocal,
        IpAddr::V6(address) if address.is_unique_local() => AddressScope::LocalNetwork,
        IpAddr::V6(address) if address.is_unicast_link_local() => AddressScope::LinkLocal,
        _ => AddressScope::Public,
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

    download_and_install(&client, &city_url, &city_path)?;
    if let Err(error) = download_and_install(&client, &asn_url, &asn_path) {
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
    decoder.read_to_end(&mut database)?;
    let reader = Reader::from_source(database.as_slice())?;
    if reader.metadata().node_count == 0 {
        return Err(GeoError::InvalidDatabase);
    }

    install_database(&database, destination)
}

fn install_database(database: &[u8], destination: &Path) -> Result<(), GeoError> {
    let pending = destination.with_extension("mmdb.new");
    let backup = destination.with_extension("mmdb.bak");
    fs::write(&pending, database)?;
    if backup.exists() {
        fs::remove_file(&backup)?;
    }
    if destination.exists() {
        fs::rename(destination, &backup)?;
    }
    if let Err(error) = fs::rename(&pending, destination) {
        if backup.exists() {
            let _ = fs::rename(&backup, destination);
        }
        return Err(error.into());
    }
    if backup.exists() {
        fs::remove_file(backup)?;
    }
    Ok(())
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

    #[test]
    fn address_scope_never_geolocates_private_ranges() {
        assert_eq!(
            classify_address("192.168.1.1".parse().expect("IP")),
            AddressScope::LocalNetwork
        );
        assert_eq!(
            classify_address("fd00::1".parse().expect("IP")),
            AddressScope::LocalNetwork
        );
        assert_eq!(
            classify_address("127.0.0.1".parse().expect("IP")),
            AddressScope::Loopback
        );
        assert_eq!(
            classify_address("1.1.1.1".parse().expect("IP")),
            AddressScope::Public
        );
    }

    #[test]
    fn geo_cache_is_bounded_and_returns_scope_without_database() {
        let mut resolver = LocalGeoResolver::empty();
        let lookup = resolver
            .lookup("10.0.0.1".parse().expect("IP"))
            .expect("lookup");
        assert_eq!(lookup.scope, AddressScope::LocalNetwork);
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
}
