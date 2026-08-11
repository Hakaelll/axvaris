import { createContext, useContext } from "react";

export type Language = "en" | "es";
export type TranslationValues = Record<string, string | number>;

const spanish: Record<string, string> = {
  Primary: "Navegación principal",
  Globe: "Globo",
  Map: "Mapa",
  Processes: "Procesos",
  Destinations: "Destinos",
  History: "Historial",
  Settings: "Ajustes",
  Language: "Idioma",
  "Interface language": "Idioma de la interfaz",
  "Changes navigation, controls, views and help text.":
    "Cambia la navegación, los controles, las vistas y los textos de ayuda.",
  "Search connections": "Buscar conexiones",
  "Search or process:firefox country:US": "Buscar o process:firefox country:ES",
  "Capture mode": "Modo de captura",
  "Capture off": "Captura desactivada",
  "Live capture": "Captura en vivo",
  "Demo data": "Datos de demostración",
  "Network interface": "Interfaz de red",
  "All interfaces": "Todas las interfaces",
  " (address unavailable)": " (dirección no disponible)",
  "Recording in progress": "Grabación en curso",
  "Start packet recording": "Iniciar grabación de paquetes",
  "Record packets": "Grabar paquetes",
  "Open command palette": "Abrir la paleta de comandos",
  "Command palette (Ctrl+K)": "Paleta de comandos (Ctrl+K)",
  "Open settings": "Abrir los ajustes",
  "LOCAL NETWORK OBSERVABILITY": "OBSERVABILIDAD DE RED LOCAL",
  "See where your computer connects.":
    "Descubre a dónde se conecta tu ordenador.",
  "Sabicheiro observes operating-system socket metadata and keeps its history on this device. It does not inspect payloads during normal capture.":
    "Sabicheiro observa los metadatos de conexiones del sistema operativo y conserva el historial en este dispositivo. Durante la captura normal no inspecciona el contenido de los paquetes.",
  "Start live capture": "Iniciar captura en vivo",
  "Explore deterministic demo": "Explorar demostración reproducible",
  "No account or telemetry": "Sin cuenta ni telemetría",
  "Local SQLite history": "Historial SQLite local",
  "Free local GeoIP": "GeoIP local y gratuito",
  "Live capture may require elevated OS permissions for complete process attribution. Demo mode never touches the network.":
    "La atribución completa de procesos puede requerir permisos elevados. El modo demostración nunca accede a la red.",
  "Live network metrics": "Métricas de red en vivo",
  ACTIVE: "ACTIVAS",
  DOWN: "BAJADA",
  UP: "SUBIDA",
  COUNTRIES: "PAÍSES",
  PROCESSES: "PROCESOS",
  PACKETS: "PAQUETES",
  "Saved local filters": "Filtros locales guardados",
  WATCHING: "SIGUIENDO",
  "Filter by {kind} {value}": "Filtrar por {kind} {value}",
  "Remove bookmark {value}": "Eliminar marcador {value}",
  "Remove local bookmark": "Eliminar marcador local",
  "Top context": "Contexto principal",
  PROCESS: "PROCESO",
  COUNTRY: "PAÍS",
  DESTINATION: "DESTINO",
  DOMAIN: "DOMINIO",
  PORT: "PUERTO",
  "NETWORK ACTIVITY": "ACTIVIDAD DE RED",
  "Applications observed opening local network sockets.":
    "Aplicaciones observadas abriendo conexiones de red locales.",
  "{count} observed": "{count} observados",
  Process: "Proceso",
  Connections: "Conexiones",
  Countries: "Países",
  Traffic: "Tráfico",
  unknown: "desconocido",
  "{count} total": "{count} en total",
  destinations: "destinos",
  "{rate} now": "{rate} ahora",
  "No processes match the current view.":
    "Ningún proceso coincide con la vista actual.",
  "REMOTE INFRASTRUCTURE": "INFRAESTRUCTURA REMOTA",
  "Domains, IP addresses, networks and countries contacted by local processes.":
    "Dominios, direcciones IP, redes y países contactados por procesos locales.",
  "Filter by country": "Filtrar por país",
  Destination: "Destino",
  Network: "Red",
  "Unresolved domain": "Dominio sin resolver",
  Unknown: "Desconocido",
  "No local GeoIP match": "Sin coincidencia GeoIP local",
  active: "activas",
  "No destinations match the current view.":
    "Ningún destino coincide con la vista actual.",
  "{active} active · {total} seen": "{active} activas · {total} observadas",
  "LOCAL EVIDENCE": "EVIDENCIA LOCAL",
  "Open retained sessions or scrub local metadata with Time Machine.":
    "Abre sesiones conservadas o recorre los metadatos locales con la Máquina del Tiempo.",
  "1 hour": "1 hora",
  "24 hours": "24 horas",
  All: "Todo",
  "Timeline zoom range": "Intervalo de la línea temporal",
  "Export view": "Exportar vista",
  "Connection activity over selected time range":
    "Actividad de conexiones durante el intervalo seleccionado",
  EARLIER: "ANTERIOR",
  "CONNECTIONS OVER TIME": "CONEXIONES A LO LARGO DEL TIEMPO",
  NOW: "AHORA",
  "Move to timeline bucket {index}: {count} connections":
    "Ir al bloque temporal {index}: {count} conexiones",
  "{count} connections": "{count} conexiones",
  "Packet recording events": "Eventos de grabación de paquetes",
  "Recording started {date}": "Grabación iniciada {date}",
  "Time Machine moment": "Momento de la Máquina del Tiempo",
  "No retained moment": "No hay un momento conservado",
  "{count} approximately active": "{count} aproximadamente activas",
  "Reconstruct globe": "Reconstruir globo",
  "All retained sessions": "Todas las sesiones conservadas",
  "Current time range": "Intervalo actual",
  sessions: "sesiones",
  "Current session": "Sesión actual",
  "connections / PCAPNG": "conexiones / PCAPNG",
  connections: "conexiones",
  "No retained sessions.": "No hay sesiones conservadas.",
  "{count} records": "{count} registros",
  "Live connections": "Conexiones en vivo",
  "LIVE CONNECTIONS": "CONEXIONES EN VIVO",
  Resume: "Reanudar",
  "Pause view": "Pausar vista",
  "Waiting for network activity": "Esperando actividad de red",
  "Start live capture or use deterministic demo data.":
    "Inicia la captura en vivo o usa los datos de demostración.",
  "Unknown location": "Ubicación desconocida",
  "Observed deviation": "Desviación observada",
  "Hide details": "Ocultar detalles",
  "Expand group": "Ampliar grupo",
  "{count} socket / {seconds}s window":
    "{count} conexión / ventana de {seconds}s",
  "{count} sockets / {seconds}s window":
    "{count} conexiones / ventana de {seconds}s",
  First: "Primera",
  Last: "Última",
  listening: "escuchando",
  connecting: "conectando",
  established: "establecida",
  closing: "cerrando",
  closed: "cerrada",
  inbound: "entrante",
  outbound: "saliente",
  "Connection inspector": "Inspector de conexión",
  "ACTIVE CONNECTION": "CONEXIÓN ACTIVA",
  "RECENT CONNECTION": "CONEXIÓN RECIENTE",
  "Close inspector": "Cerrar inspector",
  "THIS DEVICE": "ESTE DISPOSITIVO",
  Name: "Nombre",
  Executable: "Ejecutable",
  Path: "Ruta",
  "Not exposed by the operating system": "No expuesto por el sistema operativo",
  Remote: "Remoto",
  Hostname: "Nombre de host",
  Unresolved: "Sin resolver",
  Endpoint: "Extremo",
  Geo: "Geografía",
  Country: "País",
  Location: "Ubicación",
  Coordinates: "Coordenadas",
  "Not plotted": "No representado",
  "Network owner": "Propietario de la red",
  Protocol: "Protocolo",
  Local: "Local",
  Interface: "Interfaz",
  "System default": "Predeterminada del sistema",
  State: "Estado",
  Timing: "Tiempos",
  "First seen": "Primera observación",
  "Last seen": "Última observación",
  Duration: "Duración",
  Observed: "Observada",
  time: "vez",
  times: "veces",
  "First ever seen": "Primera observación histórica",
  "Checking local history...": "Consultando el historial local...",
  "Previous connections": "Conexiones anteriores",
  Sessions: "Sesiones",
  "Filter process": "Filtrar proceso",
  "Filter destination": "Filtrar destino",
  "Show country": "Mostrar país",
  "Watch process": "Seguir proceso",
  "Watch destination": "Seguir destino",
  "Ignore destination": "Ignorar destino",
  "Packets down": "Paquetes recibidos",
  "Packets up": "Paquetes enviados",
  total: "total",
  "LOCAL CONTROL": "CONTROL LOCAL",
  "Capture, storage and display preferences. Nothing is synced to a cloud service.":
    "Preferencias de captura, almacenamiento y visualización. Nada se sincroniza con servicios en la nube.",
  "Settings sections": "Secciones de ajustes",
  Capture: "Captura",
  Privacy: "Privacidad",
  "Privacy & storage": "Privacidad y almacenamiento",
  "Focus rules": "Reglas de enfoque",
  "GeoIP data": "Datos GeoIP",
  Display: "Visualización",
  Diagnostics: "Diagnóstico",
  "Native socket observation runs inside the local desktop process.":
    "La observación nativa de conexiones se ejecuta dentro del proceso local.",
  "Default network interface": "Interfaz de red predeterminada",
  "All active interfaces is the safest default.":
    "Todas las interfaces activas es la opción más fiable.",
  "All active interfaces": "Todas las interfaces activas",
  "Feed grouping window": "Ventana de agrupación del feed",
  "Coalesce the same process, remote IP, port and protocol.":
    "Agrupa el mismo proceso, IP remota, puerto y protocolo.",
  "Show local traffic": "Mostrar tráfico local",
  "Include loopback, LAN and private-address destinations in lists.":
    "Incluye destinos loopback, LAN y direcciones privadas en las listas.",
  "Show listening sockets": "Mostrar sockets en escucha",
  "Include bound TCP/UDP listeners separately from real remote connections.":
    "Incluye los listeners TCP/UDP locales por separado de las conexiones remotas reales.",
  "Live device interface total": "Total en vivo de la interfaz del equipo",
  "Observed connection total": "Total de conexiones observadas",
  "Socket metadata only": "Solo metadatos del socket",
  "Endpoint only": "Solo endpoint",
  "The current socket provider exposes endpoints but not per-connection byte counters. Live device totals are shown in the bottom bar.":
    "El proveedor actual expone los endpoints, pero no contadores de bytes por conexión. Los totales en vivo del equipo se muestran en la barra inferior.",
  "Bookmarks are quick local filters. Ignore rules affect only views and local signals; they never block traffic.":
    "Los marcadores son filtros locales rápidos. Las reglas de exclusión solo afectan a las vistas y señales locales; nunca bloquean tráfico.",
  Bookmark: "Marcador",
  "Rule action": "Acción de la regla",
  "Rule kind": "Tipo de regla",
  "Rule value": "Valor de la regla",
  "Ignore in views": "Ignorar en las vistas",
  Domain: "Dominio",
  "IP address": "Dirección IP",
  "Country code": "Código de país",
  "Localhost / LAN": "Equipo local / LAN",
  Value: "Valor",
  "Add locally": "Añadir localmente",
  Bookmarks: "Marcadores",
  Remove: "Eliminar",
  "No bookmarks yet.": "Todavía no hay marcadores.",
  "Ignored in views": "Ignorado en las vistas",
  "No traffic is explicitly ignored.": "No se ignora tráfico explícitamente.",
  "Metadata is stored in SQLite on this device. Payloads are never stored during normal capture.":
    "Los metadatos se guardan en SQLite en este dispositivo. El contenido de los paquetes nunca se almacena durante la captura normal.",
  "History retention": "Retención del historial",
  "Expired sessions are removed locally and transactionally.":
    "Las sesiones caducadas se eliminan localmente y de forma transaccional.",
  "Do not retain history": "No conservar historial",
  "1 day": "1 día",
  "7 days": "7 días",
  "30 days": "30 días",
  "90 days": "90 días",
  Unlimited: "Sin límite",
  "System reverse DNS lookups": "Consultas DNS inversas del sistema",
  "Opt in to reverse lookups through your configured system DNS resolver; remote IPs may be visible to that resolver.":
    "Activa consultas inversas mediante el DNS configurado; las IP remotas podrían ser visibles para ese resolvedor.",
  "Clearing removes the active in-memory view; retention still governs saved history.":
    "Al limpiar se elimina la vista activa en memoria; la retención sigue controlando el historial guardado.",
  "Clear session": "Limpiar sesión",
  "Local by default": "Local de forma predeterminada",
  "Sabicheiro has no analytics SDK, account, subscription, remote logging or paid API. Reverse DNS is opt-in; GeoIP lookup uses an optional local free database.":
    "Sabicheiro no tiene analítica, cuenta, suscripción, registro remoto ni API de pago. El DNS inverso es opcional y GeoIP usa una base de datos local gratuita.",
  "Optional local DB-IP Lite data. Location is approximate and never authoritative.":
    "Datos locales opcionales de DB-IP Lite. La ubicación es aproximada y nunca autoritativa.",
  "GeoIP database": "Base de datos GeoIP",
  "Version {version} · updated {date}":
    "Versión {version} · actualizada {date}",
  "Not installed; public IPs remain unlocated.":
    "No instalada; las IP públicas seguirán sin ubicación.",
  "Check free update": "Buscar actualización gratuita",
  "Install free database": "Instalar base de datos gratuita",
  "Visual preferences never change captured evidence.":
    "Las preferencias visuales nunca cambian la evidencia capturada.",
  "Reduce motion": "Reducir movimiento",
  "Disables camera focus and decorative transitions.":
    "Desactiva el enfoque de cámara y las transiciones decorativas.",
  "Smart destination focus": "Enfoque inteligente de destinos",
  "Briefly moves the globe toward a selected destination.":
    "Mueve brevemente el globo hacia el destino seleccionado.",
  "Local runtime counters; none are transmitted.":
    "Contadores locales de ejecución; ninguno se transmite.",
  BACKEND: "MOTOR",
  MEMORY: "MEMORIA",
  RENDER: "RENDERIZADO",
  "CAPTURE RATE": "TASA DE CAPTURA",
  DATABASE: "BASE DE DATOS",
  QUEUE: "COLA",
  DROPPED: "DESCARTADOS",
  "World map showing filtered network destinations":
    "Mapa mundial con destinos de red filtrados",
  "Approximate home location": "Ubicación local aproximada",
  "{count} connection to {destination}": "{count} conexión con {destination}",
  "{count} connections to {destination}":
    "{count} conexiones con {destination}",
  "Interactive globe showing filtered network destinations. Use the connection feed for a keyboard-accessible alternative.":
    "Globo interactivo con destinos de red filtrados. Usa el feed de conexiones como alternativa accesible mediante teclado.",
  "Loading local globe...": "Cargando globo local...",
  "Loading detailed local map...": "Cargando mapa local detallado...",
  "GLOBAL ROUTES": "RUTAS GLOBALES",
  "{count} active": "{count} activas",
  "{count} countries": "{count} países",
  "Visualization mode": "Modo de visualización",
  "3D globe (G)": "Globo 3D (G)",
  "WebGL unavailable": "WebGL no disponible",
  "2D map (M)": "Mapa 2D (M)",
  "TIME MACHINE": "MÁQUINA DEL TIEMPO",
  "{count} reconstructed routes": "{count} rutas reconstruidas",
  "Return to live": "Volver al directo",
  "View paused / capture continues": "Vista pausada / la captura continúa",
  "{count} updates queued": "{count} actualizaciones en cola",
  "VPN interface observed / location reflects visible remote endpoints":
    "Interfaz VPN observada / la ubicación refleja los extremos remotos visibles",
  "This device": "Este dispositivo",
  "Active destination": "Destino activo",
  "Selected route": "Ruta seleccionada",
  "Drag to rotate / scroll to zoom":
    "Arrastra para girar / desplaza para ampliar",
  "Unknown capture error": "Error de captura desconocido",
  "Capture stopped": "Captura detenida",
  "Starting local observability engine...":
    "Iniciando el motor local de observabilidad...",
  "Command palette": "Paleta de comandos",
  "Type a command...": "Escribe un comando...",
  "Open globe": "Abrir globo",
  "Switch to 3D globe": "Cambiar al globo 3D",
  "Switch to 2D map": "Cambiar al mapa 2D",
  "Open processes": "Abrir procesos",
  "Open destinations": "Abrir destinos",
  "Search process": "Buscar proceso",
  "Search destination": "Buscar destino",
  "Open history": "Abrir historial",
  "Resume visualization": "Reanudar visualización",
  "Pause visualization": "Pausar visualización",
  "Stop packet recording": "Detener grabación de paquetes",
  "Clear current session": "Limpiar sesión actual",
  "Hide local traffic": "Ocultar tráfico local",
  "Export current view": "Exportar vista actual",
  "No matching commands.": "No hay comandos coincidentes.",
  navigate: "navegar",
  select: "seleccionar",
  "Everything runs locally": "Todo se ejecuta localmente",
  "Packet recording": "Grabación de paquetes",
  "Explicit, bounded PCAPNG capture": "Captura PCAPNG explícita y limitada",
  Close: "Cerrar",
  "Recording needs finalization": "La grabación necesita finalizarse",
  "Recording packets locally": "Grabando paquetes localmente",
  "{duration} / {bytes} / limit {limit} MB per file":
    "{duration} / {bytes} / límite de {limit} MB por archivo",
  "Unlike normal observation, a packet recording can contain sensitive payloads and credentials. Only start it with authorization.":
    "A diferencia de la observación normal, una grabación puede contener datos sensibles y credenciales. Iníciala solo con autorización.",
  "Output file": "Archivo de salida",
  "Maximum size per file": "Tamaño máximo por archivo",
  "5 minutes": "5 minutos",
  "15 minutes": "15 minutos",
  "30 minutes": "30 minutos",
  "Rotate recording files": "Rotar archivos de grabación",
  "Keep a bounded ring of at most four files.":
    "Mantiene un anillo limitado de cuatro archivos como máximo.",
  "Windows uses the built-in free Pktmon tool when available. Linux requires a locally installed free capture helper and appropriate permissions.":
    "Windows usa la herramienta gratuita Pktmon incluida cuando está disponible. Linux requiere un capturador gratuito instalado localmente y los permisos adecuados.",
  Cancel: "Cancelar",
  "Retry finalization": "Reintentar finalización",
  "Stop recording": "Detener grabación",
  "Start recording": "Iniciar grabación",
  "Metadata only; packet payloads are never included":
    "Solo metadatos; nunca se incluye el contenido de los paquetes",
  "For spreadsheets and analysis": "Para hojas de cálculo y análisis",
  "Structured full-fidelity records": "Registros estructurados completos",
  Scope: "Ámbito",
  "Filter: {query}": "Filtro: {query}",
  "Session: {session}": "Sesión: {session}",
  "Selected history range": "Intervalo de historial seleccionado",
  "All visible records": "Todos los registros visibles",
  "Export {format}": "Exportar {format}",
  "Operation could not be completed": "No se pudo completar la operación",
  "Dismiss error": "Descartar error",
  "capture provider is unavailable":
    "el proveedor de captura no está disponible",
};

const I18nContext = createContext<Language>("en");

export const I18nProvider = I18nContext.Provider;

function interpolate(message: string, values: TranslationValues): string {
  return message.replace(/\{(\w+)\}/g, (placeholder, key: string) =>
    key in values ? String(values[key]) : placeholder,
  );
}

function translateDynamicSpanish(source: string): string | null {
  const missingAddress = source.match(
    /^network interface '(.+)' has no discoverable IP addresses; use All interfaces$/,
  );
  if (missingAddress) {
    return `la interfaz de red '${missingAddress[1]}' no tiene direcciones IP detectables; usa Todas las interfaces`;
  }
  const missingInterface = source.match(
    /^network interface '(.+)' is no longer available$/,
  );
  if (missingInterface) {
    return `la interfaz de red '${missingInterface[1]}' ya no está disponible`;
  }
  return null;
}

export function translate(
  language: Language,
  source: string,
  values: TranslationValues = {},
): string {
  const message =
    language === "es"
      ? (spanish[source] ?? translateDynamicSpanish(source) ?? source)
      : source;
  return interpolate(message, values);
}

export function useI18n(): (
  source: string,
  values?: TranslationValues,
) => string {
  const language = useContext(I18nContext);
  return (source, values) => translate(language, source, values);
}

export function useLanguage(): Language {
  return useContext(I18nContext);
}
