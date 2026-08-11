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
  "Connection workspace": "Espacio de conexiones",
  Listeners: "Servicios en escucha",
  Notifications: "Notificaciones",
  "Local notifications": "Notificaciones locales",
  "Clear all": "Limpiar todo",
  "Dismiss notification": "Descartar notificación",
  "Separate connection workspaces": "Espacios de conexiones separados",
  "Remote peers, local traffic and listeners are always kept in separate tabs so their counts and meaning are never mixed.":
    "Los destinos remotos, el tráfico local y los servicios en escucha permanecen en pestañas separadas para no mezclar sus recuentos ni su significado.",
  "Opt-in, explainable rules are evaluated locally and rate-limited. They never block traffic.":
    "Las reglas opcionales y explicables se evalúan localmente y tienen un límite de frecuencia. Nunca bloquean tráfico.",
  "Notification rule": "Regla de notificación",
  "New network process": "Nuevo proceso con actividad de red",
  "New destination country": "Nuevo país de destino",
  "ASN number or organization": "Número u organización ASN",
  "Remote port": "Puerto remoto",
  "Notification value": "Valor de la notificación",
  "Add notification": "Añadir notificación",
  "Any new observation": "Cualquier observación nueva",
  "Remove notification rule": "Eliminar regla de notificación",
  "No notification rules yet.": "Todavía no hay reglas de notificación.",
  "ASN rule matched": "Coincidencia con regla ASN",
  "Port rule matched": "Coincidencia con regla de puerto",
  new_process: "nuevo proceso",
  new_country: "nuevo país",
  asn: "ASN",
  port: "puerto",
  "Diagnostic support bundle": "Paquete de soporte de diagnóstico",
  "Review and export versions, redacted settings and local logs. Connection history, raw IPs, payloads, recordings and secrets are excluded.":
    "Revisa y exporta versiones, ajustes anonimizados y registros locales. Se excluyen el historial de conexiones, las IP sin anonimizar, los contenidos, las grabaciones y los secretos.",
  "Review exactly what is included: versions and redacted diagnostics, with optional redacted settings and local application logs.":
    "Revisa exactamente qué se incluye: versiones y diagnóstico anonimizado, con ajustes anonimizados y registros locales opcionales.",
  "Support bundle filename": "Nombre del paquete de soporte",
  "Redacted settings": "Ajustes anonimizados",
  "Local application logs": "Registros locales de la aplicación",
  "Export reviewed ZIP": "Exportar ZIP revisado",
  "Saved {count} redacted files to {path}":
    "Se guardaron {count} archivos anonimizados en {path}",
  "Offline MMDB import": "Importación MMDB sin conexión",
  "Import a free City or ASN database from this computer. Edition and checksum are verified before the local atomic replacement.":
    "Importa una base City o ASN gratuita desde este equipo. Se verifican la edición y la suma de comprobación antes del reemplazo local atómico.",
  "City MMDB": "MMDB de ciudades",
  "ASN MMDB": "MMDB de ASN",
  "IP geolocation by DB-IP.com, licensed under CC BY 4.0":
    "Geolocalización IP de DB-IP.com, con licencia CC BY 4.0",
  "Choose a DB-IP Lite or compatible City/ASN MMDB already on this device. Sabicheiro validates its edition and checksum before an atomic local install.":
    "Elige una base DB-IP Lite o una MMDB City/ASN compatible ya presente en este equipo. Sabicheiro valida la edición y la suma de verificación antes de instalarla localmente de forma atómica.",
  "MMDB kind": "Tipo de MMDB",
  "MMDB file path": "Ruta del archivo MMDB",
  "Verify and import": "Verificar e importar",
  EDITION: "EDICIÓN",
  Available: "Disponible",
  Unavailable: "No disponible",
  "EVENT SOURCE": "ORIGEN DE EVENTOS",
  "FLOW COUNTERS": "CONTADORES POR FLUJO",
  "PACKET EVENTS": "EVENTOS DE PAQUETES",
  "DNS CORRELATIONS": "CORRELACIONES DNS",
  "Capture self-test": "Autoprueba de captura",
  "Checks adapters, socket tables, process attribution, DNS, GeoIP and storage without requiring a terminal.":
    "Comprueba adaptadores, tablas de sockets, atribución de procesos, DNS, GeoIP y almacenamiento sin usar una terminal.",
  "Running checks...": "Ejecutando comprobaciones...",
  "Run self-test": "Ejecutar autoprueba",
  "Network interfaces are discoverable":
    "Las interfaces de red son detectables",
  "No active addressed interface was found":
    "No se encontró ninguna interfaz activa con dirección",
  "Interface discovery failed": "Falló la detección de interfaces",
  "Socket tables are readable": "Las tablas de sockets se pueden leer",
  "Process attribution is working": "La atribución de procesos funciona",
  "The proof socket could not be attributed":
    "No se pudo atribuir el socket de prueba",
  "The current Sabicheiro process was resolved from a real loopback connection.":
    "El proceso actual de Sabicheiro se identificó mediante una conexión real de bucle local.",
  "Socket visibility works, but the operating system did not expose the proof PID in time.":
    "Los sockets son visibles, pero el sistema operativo no expuso a tiempo el PID de prueba.",
  "Socket table proof failed": "Falló la prueba de la tabla de sockets",
  "Passive DNS correlation is active": "La correlación DNS pasiva está activa",
  "Opt-in reverse DNS is enabled": "El DNS inverso opcional está activado",
  "DNS enrichment is intentionally idle":
    "El enriquecimiento DNS está inactivo intencionadamente",
  "Windows DNS Client answers are correlated locally with an expiring bounded cache.":
    "Las respuestas del cliente DNS de Windows se correlacionan localmente mediante una caché limitada y con caducidad.",
  "Start Live capture with ETW or explicitly enable reverse DNS to enrich domains.":
    "Inicia la captura en vivo con ETW o activa expresamente el DNS inverso para enriquecer los dominios.",
  "Local GeoIP database is ready": "La base GeoIP local está lista",
  "Local GeoIP database is not installed":
    "La base GeoIP local no está instalada",
  "Download the free dataset or import a City MMDB from Settings; observed IPs are never uploaded.":
    "Descarga el conjunto gratuito o importa una MMDB City desde Ajustes; las IP observadas nunca se suben.",
  "Local history storage is available": "El historial local está disponible",
  "Development preview adapter is available.":
    "El adaptador de la vista previa de desarrollo está disponible.",
  "Development preview uses deterministic local data.":
    "La vista previa de desarrollo usa datos locales reproducibles.",
  "Preview hostnames are deterministic and local.":
    "Los nombres de host de la vista previa son reproducibles y locales.",
  "Development preview dataset.":
    "Conjunto de datos de la vista previa de desarrollo.",
  "TRAFFIC TIMELINE": "LÍNEA TEMPORAL DE TRÁFICO",
  "Stored rates / {source}": "Tasas guardadas / {source}",
  "Traffic resolution": "Resolución del tráfico",
  "Resume timeline": "Reanudar línea temporal",
  "Pause timeline": "Pausar línea temporal",
  "Traffic rates over selected time range":
    "Tasas de tráfico durante el intervalo seleccionado",
  "No traffic samples in this range.":
    "No hay muestras de tráfico en este intervalo.",
  "SESSION COMPARISON": "COMPARACIÓN DE SESIONES",
  "Compare retained evidence": "Comparar evidencia conservada",
  "Compare processes, destinations, countries and ports without leaving local storage.":
    "Compara procesos, destinos, países y puertos sin salir del almacenamiento local.",
  "Earlier session": "Sesión anterior",
  "Later session": "Sesión posterior",
  "Compare sessions": "Comparar sesiones",
  "Comparison category": "Categoría de comparación",
  processes: "procesos",
  countries: "países",
  ports: "puertos",
  "Saved comparison to {path}": "Comparación guardada en {path}",
  Earlier: "Anterior",
  Later: "Posterior",
  "Recent process traffic sparkline":
    "Minigráfico de tráfico reciente del proceso",
  "per-flow events": "eventos por flujo",
  "interface aggregate only": "solo total de interfaz",
  "Geography is not available yet": "La geografía todavía no está disponible",
  "Connections are still captured. Install or import a free local City MMDB to place public destinations on the map.":
    "Las conexiones se siguen capturando. Instala o importa una MMDB City local y gratuita para situar los destinos públicos en el mapa.",
  "Open GeoIP settings": "Abrir ajustes de GeoIP",
  "Original diagnostic detail": "Detalle de diagnóstico original",
  "New network-enabled process": "Nuevo proceso con acceso a red",
  "New destination": "Nuevo destino",
  "Higher destination diversity": "Mayor diversidad de destinos",
  "Unusual for this process": "Inusual para este proceso",
  "Unusual port": "Puerto inusual",
  "Unusual activity": "Actividad inusual",
  "Higher bandwidth than usual": "Ancho de banda superior al habitual",
  "Persistent connection": "Conexión persistente",
  "This is an observation, not a security verdict.":
    "Esto es una observación, no un veredicto de seguridad.",
  "Previously observed destination set: 11 or fewer.":
    "Conjunto de destinos observado anteriormente: 11 o menos.",
  "Typical observed range is below 25 in four seconds.":
    "El intervalo observado habitual es inferior a 25 en cuatro segundos.",
  "Long-lived connections can be normal for messaging, media and sync applications.":
    "Las conexiones duraderas pueden ser normales en aplicaciones de mensajería, multimedia y sincronización.",
  "Network capture is unavailable. Try All interfaces or run the capture self-test.":
    "La captura de red no está disponible. Prueba Todas las interfaces o ejecuta la autoprueba de captura.",
  "The selected network interface is unavailable. Choose All interfaces and try again.":
    "La interfaz de red seleccionada no está disponible. Elige Todas las interfaces e inténtalo de nuevo.",
  "Local history could not be read. Check Diagnostics for the original storage detail.":
    "No se pudo leer el historial local. Consulta Diagnóstico para ver el detalle original del almacenamiento.",
  "Stored traffic samples could not be read for this time range.":
    "No se pudieron leer las muestras de tráfico guardadas para este intervalo.",
  "Local storage is unavailable. Sabicheiro has kept the original diagnostic below.":
    "El almacenamiento local no está disponible. Sabicheiro conserva debajo el diagnóstico original.",
  "The free GeoIP update could not be installed. You can retry or import an MMDB offline.":
    "No se pudo instalar la actualización GeoIP gratuita. Puedes reintentarlo o importar una MMDB sin conexión.",
  "The MMDB import failed edition, checksum or filesystem validation.":
    "La importación MMDB no superó la validación de edición, suma de verificación o sistema de archivos.",
  "The local export could not be written. Choose a new filename or writable folder.":
    "No se pudo escribir la exportación local. Elige otro nombre o una carpeta con permiso de escritura.",
  "The redacted support bundle could not be created.":
    "No se pudo crear el paquete de soporte anonimizado.",
  "The selected sessions could not be compared from local history.":
    "No se pudieron comparar las sesiones seleccionadas desde el historial local.",
  "Both selected sessions need retained connections.":
    "Las dos sesiones seleccionadas deben contener conexiones conservadas.",
  "That output already exists. Choose a new filename.":
    "Ese archivo ya existe. Elige otro nombre.",
  "The output filename or extension is invalid.":
    "El nombre o la extensión del archivo de salida no son válidos.",
  "One of the local settings is outside its allowed range.":
    "Uno de los ajustes locales está fuera del intervalo permitido.",
  "The operation could not be completed.": "No se pudo completar la operación.",
  "The capture backend stopped. Run the self-test or select a different interface.":
    "El motor de captura se detuvo. Ejecuta la autoprueba o selecciona otra interfaz.",
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
  const firstProcess = source.match(
    /^This is the first locally retained network connection for (.+)\.$/,
  );
  if (firstProcess) {
    return `Esta es la primera conexión de red conservada localmente para ${firstProcess[1]}.`;
  }
  const firstDestination = source.match(
    /^First observed connection from (.+) to (.+)\.$/,
  );
  if (firstDestination) {
    return `Primera conexión observada de ${firstDestination[1]} a ${firstDestination[2]}.`;
  }
  const diverseDestinations = source.match(
    /^(.+) has now contacted 12 distinct destinations in the retained local baseline\.$/,
  );
  if (diverseDestinations) {
    return `${diverseDestinations[1]} ya ha contactado con 12 destinos distintos en la referencia local conservada.`;
  }
  const previousCountries = source.match(/^Previously observed: (.+)$/);
  if (previousCountries) {
    return `Observado anteriormente: ${previousCountries[1]}`;
  }
  const unusualPort = source.match(
    /^(.+) has not previously used remote port (\d+)\.$/,
  );
  if (unusualPort) {
    return `${unusualPort[1]} no había usado antes el puerto remoto ${unusualPort[2]}.`;
  }
  const burst = source.match(
    /^(.+) created 25 connections within four seconds\.$/,
  );
  if (burst) {
    return `${burst[1]} creó 25 conexiones en cuatro segundos.`;
  }
  const bandwidth = source.match(
    /^(.+) transferred approximately ([\d.]+) MiB\/s in this interval\.$/,
  );
  if (bandwidth) {
    return `${bandwidth[1]} transfirió aproximadamente ${bandwidth[2]} MiB/s en este intervalo.`;
  }
  const retainedMean = source.match(/^Retained local mean: ([\d.]+) KiB\/s\.$/);
  if (retainedMean) {
    return `Media local conservada: ${retainedMean[1]} KiB/s.`;
  }
  const persistent = source.match(
    /^(.+) has kept this connection open for at least 30 minutes\.$/,
  );
  if (persistent) {
    return `${persistent[1]} ha mantenido esta conexión abierta al menos 30 minutos.`;
  }
  const adapterCount = source.match(
    /^(\d+) adapters found with stable local identifiers$/,
  );
  if (adapterCount) {
    return `${adapterCount[1]} adaptadores encontrados con identificadores locales estables`;
  }
  const unavailableAdapters = source.match(
    /^(\d+) adapters were returned; reconnect the network or select All interfaces$/,
  );
  if (unavailableAdapters) {
    return `Se devolvieron ${unavailableAdapters[1]} adaptadores; vuelve a conectar la red o selecciona Todas las interfaces`;
  }
  const socketRows = source.match(
    /^(\d+) TCP\/UDP rows observed during a local loopback proof$/,
  );
  if (socketRows) {
    return `${socketRows[1]} filas TCP/UDP observadas durante una prueba local de bucle`;
  }
  const geoChecksum = source.match(/^(.+) \/ checksum (.+)$/);
  if (geoChecksum) {
    return `${geoChecksum[1]} / suma de verificación ${geoChecksum[2]}`;
  }
  const sqliteSize = source.match(
    /^SQLite is writable; current file size is (\d+) bytes$/,
  );
  if (sqliteSize) {
    return `SQLite permite escritura; el archivo actual ocupa ${sqliteSize[1]} bytes`;
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
