import { createContext, useContext } from "react";

export type Language = "en" | "es";
export type TranslationValues = Record<string, string | number>;

const spanish: Record<string, string> = {
  "Observed {time}": "Observado a las {time}",
  "Open the retained evidence to inspect the destination's source and quality.":
    "Abre la evidencia guardada para consultar el origen y la calidad del destino.",
  "New investigation": "Nueva investigación",
  "Edit investigation": "Editar investigación",
  Time: "Hora",
  Type: "Tipo",
  Result: "Resultado",
  Current: "Actual",
  "Added listener": "Añadido",
  "Removed listener": "Eliminado",
  "Snapshot comparison": "Comparación de snapshots",
  "History views": "Vistas del historial",
  Workspace: "Espacio de trabajo",
  Timeline: "Línea temporal",
  "Selected connection": "Conexión seleccionada",
  "Evidence details": "Detalles de la evidencia",
  "System architecture": "Arquitectura del sistema",
  "Supporting components": "Componentes auxiliares",
  "Previous snapshot": "Snapshot anterior",
  "Current snapshot": "Snapshot actual",
  Present: "Presente",
  Absent: "Ausente",
  added: "añadidos",
  removed: "eliminados",
  "Quick comparison": "Comparación rápida",
  "Location coverage": "Cobertura de ubicación",
  "Local endpoint": "Endpoint local",
  "Showing {shown} of {count} sockets":
    "Se muestran {shown} de {count} sockets",
  "Select a service to inspect its socket evidence":
    "Selecciona un servicio para consultar la evidencia de su socket",
  "Unknown interface": "Interfaz desconocida",
  "MAC address": "Dirección MAC",
  "Start live capture": "Iniciar captura en vivo",
  "Use demo data": "Usar datos de demostración",
  "Raises text and border contrast and makes glass controls opaque.":
    "Aumenta el contraste del texto y los bordes y hace opacos los controles de cristal.",
  "Capture health": "Salud de captura",
  Application: "Aplicación",
  "Network profile": "Perfil de red",
  "Local processing": "Procesamiento local",
  "External DNS evidence": "Evidencia DNS externa",
  "Import resolver logs to follow clients and queried domains. DNS queries do not prove a connection or identify a remote process.":
    "Importa registros DNS para seguir clientes y dominios. Las consultas no confirman conexiones ni identifican procesos remotos.",
  "Source name": "Nombre de la fuente",
  "Home resolver": "Servidor DNS doméstico",
  "CSV or JSON file": "Archivo CSV o JSON",
  "CSV template": "Plantilla CSV",
  "JSON template": "Plantilla JSON",
  "The import exceeds the 16 MiB limit.":
    "La importación supera el límite de 16 MiB.",
  "Timestamps must include a timezone. Imports stay local and are deduplicated per source. Limit: 16 MiB / 100,000 queries.":
    "Las fechas deben incluir zona horaria. Los datos permanecen locales y se deduplican por fuente. Límite: 16 MiB / 100.000 consultas.",
  "Validating DNS queries...": "Validando consultas DNS...",
  "Importing DNS queries...": "Importando consultas DNS...",
  "Cancel import": "Cancelar importación",
  "Imported {inserted} queries; skipped {duplicates} duplicates.":
    "Se importaron {inserted} consultas y se omitieron {duplicates} duplicados.",
  "{count} valid queries. Review the sample before importing.":
    "{count} consultas válidas. Revisa la muestra antes de importar.",
  "Import queries": "Importar consultas",
  "Discard preview": "Descartar vista previa",
  "Client IP": "IP del cliente",
  "Domain contains": "El dominio contiene",
  From: "Desde",
  To: "Hasta",
  "Apply filters": "Aplicar filtros",
  "DNS query evidence": "Evidencia de consultas DNS",
  "Import preview": "Vista previa de importación",
  "Imported DNS history": "Historial DNS importado",
  "No imported queries match. Import a file or adjust your filters.":
    "No hay consultas importadas que coincidan. Importa un archivo o ajusta los filtros.",
  "Next page": "Página siguiente",
  "City database not installed": "Base City no instalada",
  "Installing City database...": "Instalando la base City...",
  "No map coordinates for these destinations":
    "Sin coordenadas para estos destinos",
  "The local City database has no coordinates for the visible addresses. Inspect each connection for available country data.":
    "La base City local no tiene coordenadas para las direcciones visibles. Inspecciona cada conexión para consultar los datos de país disponibles.",
  "Connections are captured, but public locations need a local City database. Install the free dataset to place matching destinations on the map.":
    "Las conexiones se capturan, pero las ubicaciones públicas necesitan una base City local. Instala la base gratuita para situar los destinos con coincidencia en el mapa.",
  "Review unlocated targets": "Revisar destinos sin ubicación",
  "Local address — no public location":
    "Dirección local: sin ubicación pública",
  "No match in City database": "Sin coincidencia en la base City",
  "LOCAL DEVICES": "DISPOSITIVOS LOCALES",
  "Local devices": "Dispositivos locales",
  "{count} device groups": "{count} grupos de dispositivos",
  "Devices are grouped only when the operating system has a link address for them. The list uses existing neighbor and connection records; it does not scan the network.":
    "Solo se agrupan dispositivos cuando el sistema conoce su dirección física. La lista usa vecinos y conexiones ya observados; no escanea la red.",
  "Devices with MAC evidence": "Dispositivos con dirección física",
  "Addresses without device identity":
    "Direcciones sin identidad de dispositivo",
  "Grouped by MAC address": "Agrupado por dirección MAC",
  "Address only — device unverified":
    "Solo dirección: dispositivo sin verificar",
  "No local devices are known yet. Start live capture or communicate with a device on this network.":
    "Aún no se conocen dispositivos locales. Inicia la captura en vivo o comunícate con un equipo de esta red.",
  "No connection from this computer observed":
    "No se ha observado ninguna conexión desde este ordenador",
  "Checking devices...": "Consultando dispositivos...",
  "Refresh devices": "Actualizar dispositivos",
  "Neighbor cache": "Caché de vecinos",
  "Neighbor cache unavailable": "Caché de vecinos no disponible",
  "Observed connections remain listed below.":
    "Las conexiones observadas siguen mostrándose abajo.",
  "Loading local devices...": "Cargando dispositivos locales...",
  "Select a device to inspect its connections":
    "Selecciona un dispositivo para inspeccionar sus conexiones",
  Devices: "Dispositivos",
  Device: "Dispositivo",
  Router: "Router",
  "Recently observed": "Observado recientemente",
  Active: "Activo",
  Ports: "Puertos",
  "Showing the 100 most recent peers of {count}.":
    "Se muestran los 100 equipos más recientes de {count}.",
  Today: "Hoy",
  Live: "En vivo",
  Investigate: "Investigar",
  Exposure: "Exposición",
  "Data & Health": "Datos y estado",
  ".sabi file path": "Ruta del archivo .sabi",
  "{column} width": "Ancho de {column}",
  "{count} DNS candidates; none promoted to confirmed":
    "{count} candidatos DNS; ninguno se ha promovido a confirmado",
  "{count} dry-run matches": "{count} coincidencias en la simulación",
  "{count} exact rows": "{count} filas exactas",
  "{count} grouped events": "{count} eventos agrupados",
  "{count} listeners": "{count} listeners",
  "{count} local profiles": "{count} perfiles locales",
  "{count} retained": "{count} conservados",
  "{count} versioned rules": "{count} reglas versionadas",
  "A rule needs a condition and at least one safe action.":
    "Una regla necesita una condición y al menos una acción segura.",
  "Accessibility rendering mode": "Modo de representación accesible",
  Action: "Acción",
  Actions: "Acciones",
  "Add at least one typed condition.": "Añade al menos una condición tipada.",
  "Add chip": "Añadir chip",
  "Add condition": "Añadir condición",
  "Add grouped": "Añadir agrupado",
  "added since previous snapshot": "añadidos desde el snapshot anterior",
  "AND requires every condition; OR accepts either group. Negation excludes matching evidence.":
    "AND exige todas las condiciones; OR acepta cualquiera de los grupos. La negación excluye la evidencia coincidente.",
  "ASN is local dataset context, not a security verdict.":
    "El ASN es contexto del dataset local, no un veredicto de seguridad.",
  "Background low-resource": "Segundo plano de bajo consumo",
  "Bind endpoint": "Endpoint de escucha",
  "Build an evidence query": "Construir una consulta de evidencia",
  Canonicalize: "Normalizar",
  "Capability degradation": "Degradación de capacidad",
  "Capture signed snapshot": "Capturar snapshot firmado",
  "Capture the first local listener snapshot to establish a comparison point.":
    "Captura el primer snapshot local de listeners para establecer un punto de comparación.",
  "Capturing local snapshot…": "Capturando snapshot local…",
  "Causal context": "Contexto causal",
  Change: "Cambio",
  "Changes are observations with traceable evidence, never malware verdicts.":
    "Los cambios son observaciones con evidencia trazable, nunca veredictos de malware.",
  "Changes worth reviewing": "Cambios que merece la pena revisar",
  "Checking native route…": "Consultando la ruta nativa…",
  "Choose a previous query": "Elegir una consulta anterior",
  "Close bundle": "Cerrar paquete",
  "Columns and widths": "Columnas y anchos",
  conditions: "condiciones",
  "Current and changed local listener evidence":
    "Evidencia actual y modificada de listeners locales",
  "Current network profile": "Perfil de red actual",
  Custom: "Personalizado",
  Disabled: "Desactivada",
  "Disabled by default. Enabling records explicit intent; platform startup registration is reported separately in diagnostics.":
    "Desactivado por defecto. Al habilitarlo se registra la intención explícita; el registro de inicio de la plataforma se informa por separado en Diagnóstico.",
  Dismiss: "Descartar",
  "DNS limitation": "Limitación DNS",
  "DNS relation": "Relación DNS",
  "DoH, DoT or a prior shared cache may hide the originating query.":
    "DoH, DoT o una caché compartida previa pueden ocultar la consulta de origen.",
  "Dry-run completed locally.": "Simulación local completada.",
  "Duplicate rule: {name}": "Regla duplicada: {name}",
  "Edit as next version": "Editar como nueva versión",
  "Editable query chips": "Chips editables de consulta",
  "Enable bounded handshake metadata until you turn it off? Buffers are discarded and application payload is never persisted.":
    "¿Activar metadatos acotados de handshake hasta que los desactives? Los búferes se descartan y el payload de aplicación nunca se conserva.",
  Enabled: "Activada",
  "Enforce offline mode": "Forzar modo sin conexión",
  "Enhanced contrast": "Contraste reforzado",
  "Enter a valid value for the selected condition.":
    "Introduce un valor válido para la condición seleccionada.",
  Ephemeral: "Efímero",
  "Ephemeral handshake metadata": "Metadatos efímeros de handshake",
  "Evidence pages": "Páginas de evidencia",
  "Executable revision": "Revisión del ejecutable",
  "Export redacted .sabi": "Exportar .sabi redactado",
  "Export redacted evidence": "Exportar evidencia redactada",
  "Exposure could not be displayed": "No se pudo mostrar Exposición",
  "Exported {count} records to {path}":
    "Se exportaron {count} registros a {path}",
  Field: "Campo",
  "Fields, operators and observed values are completed entirely on this device.":
    "Los campos, operadores y valores observados se completan totalmente en este dispositivo.",
  "Firewall evidence": "Evidencia del firewall",
  "Firewall state is reported as not queryable until a native read-only rule provider supplies evidence. Bind scope alone never becomes a vulnerability verdict.":
    "El estado del firewall se muestra como no consultable hasta que un proveedor nativo de solo lectura aporte evidencia. El alcance del bind por sí solo nunca se convierte en un veredicto de vulnerabilidad.",
  "First observed opening a network connection on this device.":
    "Primera observación abriendo una conexión de red en este dispositivo.",
  "FIRST-CLASS EVIDENCE TABLE": "TABLA DE EVIDENCIA PRINCIPAL",
  "Fix the positional error before applying this query.":
    "Corrige el error indicado antes de aplicar esta consulta.",
  "Gateway pseudonym": "Seudónimo del gateway",
  Group: "Agrupar",
  "HANDSHAKE METADATA MODE IS ACTIVE":
    "EL MODO DE METADATOS DE HANDSHAKE ESTÁ ACTIVO",
  "Hash-signed local snapshots": "Snapshots locales firmados con hash",
  "Imported read-only investigation": "Investigación importada de solo lectura",
  "Interface unavailable": "Interfaz no disponible",
  "Investigation high-fidelity": "Investigación de alta fidelidad",
  "Investigation tags": "Etiquetas de la investigación",
  "IP evidence remains valid": "La evidencia IP sigue siendo válida",
  Join: "Unión",
  Latest: "Más reciente",
  "Loading listener snapshots…": "Cargando snapshots de listeners…",
  "Listener evidence": "Evidencia de listeners",
  "listeners in latest snapshot": "listeners en el snapshot más reciente",
  "LOCAL DECISION SUMMARY": "RESUMEN LOCAL PARA DECIDIR",
  "Local history": "Historial local",
  "LOCAL LISTENER AUDIT": "AUDITORÍA LOCAL DE LISTENERS",
  "Local notification inbox": "Bandeja local de notificaciones",
  "LOCAL QUERY BUILDER": "CONSTRUCTOR LOCAL DE CONSULTAS",
  Meaning: "Significado",
  Negate: "Negar",
  "Network path": "Camino de red",
  "New domain": "Dominio nuevo",
  "New network owner hint": "Nueva pista de propietario de red",
  "New workspace": "Nuevo workspace",
  Next: "Siguiente",
  "No causal DNS evidence": "Sin evidencia DNS causal",
  "No conditions yet": "Todavía no hay condiciones",
  "No DNS evidence": "Sin evidencia DNS",
  "No listener evidence in the latest snapshot":
    "Sin evidencia de listeners en el último snapshot",
  "No local notes in this bundle.": "Este paquete no contiene notas locales.",
  "No local notifications.": "No hay notificaciones locales.",
  "No new retained changes to review":
    "No hay cambios nuevos conservados que revisar",
  "No offline catalog hint": "Sin pista del catálogo offline",
  "No process instance evidence": "Sin evidencia de instancia del proceso",
  "No retained evidence matches this query.":
    "Ninguna evidencia conservada coincide con esta consulta.",
  None: "Ninguno",
  "None reported": "No se informó de ninguna",
  NOT: "NO",
  "Notify locally": "Notificar localmente",
  "Observed or correlated name": "Nombre observado o correlacionado",
  "Observed port / protocol": "Puerto/protocolo observado",
  "Observed value": "Valor observado",
  Open: "Abrir",
  "Open a change to inspect its retained connection evidence.":
    "Abre cualquier tarjeta para llegar a su evidencia concreta de conexión en un paso.",
  "Open Data & Health": "Abrir Datos y estado",
  "Open evidence": "Abrir evidencia",
  "Open Exposure": "Abrir Exposición",
  "Open Investigate": "Abrir Investigar",
  "Open Live": "Abrir En vivo",
  "Open portable investigation": "Abrir investigación portable",
  "Open row for native route evidence":
    "Abre la fila para consultar la evidencia de ruta nativa",
  "Open Today": "Abrir Hoy",
  "Operation profile": "Perfil de operación",
  "Optional, bounded and non-persistent TLS/QUIC metadata only: version, SNI and ALPN when observable. No payload, decryption, keys or MITM.":
    "Solo metadatos TLS/QUIC opcionales, acotados y no persistentes: versión, SNI y ALPN cuando sean observables. Sin payload, descifrado, claves ni MITM.",
  "Page {page} of {pages}; {count} exact rows":
    "Página {page} de {pages}; {count} filas exactas",
  Pin: "Fijar",
  "Pin matching evidence": "Fijar la evidencia coincidente",
  Preset: "Preajuste",
  "Preview on retained history": "Previsualizar sobre el historial conservado",
  Previous: "Anterior",
  "Process / instance": "Proceso/instancia",
  "Profiles change capture cadence and resource budgets; the active behavior remains visible.":
    "Los perfiles cambian la cadencia de captura y los presupuestos de recursos; el comportamiento activo permanece visible.",
  "Reachability limitation": "Limitación de alcance",
  "Reload Exposure": "Volver a cargar Exposición",
  "READ-ONLY .SABI / schema {version}":
    ".SABI DE SOLO LECTURA / esquema {version}",
  "Registrable domain": "Dominio registrable",
  "Rejects every optional network action, including GeoIP downloads and reverse DNS. Offline imports remain available.":
    "Rechaza todas las acciones de red opcionales, incluidas las descargas GeoIP y el DNS inverso. La importación offline sigue disponible.",
  "Remove this condition": "Eliminar esta condición",
  "removed since previous snapshot": "eliminados desde el snapshot anterior",
  "Require every condition (AND)": "Exigir todas las condiciones (AND)",
  Resolve: "Resolver",
  "Revision unavailable": "Revisión no disponible",
  "Route evidence": "Evidencia de ruta",
  "Route limitation": "Limitación de ruta",
  "Route metric": "Métrica de ruta",
  "Rule name": "Nombre de la regla",
  "Rule version {version} saved locally.":
    "La versión {version} de la regla se guardó localmente.",
  "Rules and labels": "Reglas y etiquetas",
  "Rules may tag, pin, add to an investigation or notify. They cannot block traffic, run commands or start packet capture.":
    "Las reglas pueden etiquetar, fijar, añadir a una investigación o notificar. No pueden bloquear tráfico, ejecutar comandos ni iniciar capturas de paquetes.",
  "Safe actions": "Acciones seguras",
  "SAFE LOCAL AUTOMATION": "AUTOMATIZACIÓN LOCAL SEGURA",
  "Save new version": "Guardar nueva versión",
  "Save now": "Guardar ahora",
  "Save preset locally": "Guardar preajuste localmente",
  "Saved exact time window": "Ventana temporal exacta guardada",
  "Saved local preset": "Preajuste local guardado",
  "Screen reader pages": "Páginas para lector de pantalla",
  "Selected retained session": "Sesión conservada seleccionada",
  "SHA-256 covers the deterministic listener set stored in SQLite.":
    "SHA-256 cubre el conjunto determinista de listeners guardado en SQLite.",
  "Snooze 1 h": "Posponer 1 h",
  "Source path": "Ruta de origen",
  "Some malformed listener records were ignored to keep Exposure available.":
    "Se ignoraron algunos registros de listeners no válidos para mantener disponible Exposición.",
  "Start capture or use Demo; provider limitations remain visible in Data & Health.":
    "Inicia la captura o usa Demo; las limitaciones del proveedor siguen visibles en Datos y estado.",
  "Start Axvaris when I sign in": "Iniciar Axvaris al acceder a mi sesión",
  Tag: "Etiqueta",
  "Tags separated by commas": "Etiquetas separadas por comas",
  text: "texto",
  "Text scale": "Escala del texto",
  "The exact filtered IDs used by timeline, map and export.":
    "Los mismos ID filtrados que usan el timeline, el mapa y la exportación.",
  "The view isolated an unexpected rendering error. Network capture and retained evidence remain available.":
    "La vista aisló un error de renderizado inesperado. La captura de red y la evidencia conservada siguen disponibles.",
  "The ZIP manifest, exact entry set, CRC and SHA-256 hashes are verified before opening. Imported evidence stays read-only and is never merged or uploaded.":
    "Antes de abrir se verifican el manifiesto ZIP, el conjunto exacto de entradas, el CRC y los hashes SHA-256. La evidencia importada permanece en solo lectura y nunca se mezcla ni se sube.",
  "This is an observation result, not proof that no service can listen between snapshots.":
    "Este es un resultado de observación, no una prueba de que ningún servicio pueda escuchar entre snapshots.",
  "Timeline samples": "Muestras del timeline",
  "Typed field": "Campo tipado",
  unchanged: "sin cambios",
  "unknown interface": "interfaz desconocida",
  "Unknown process": "Proceso desconocido",
  Unpin: "Desfijar",
  "Untitled local rule": "Regla local sin título",
  User: "Usuario",
  "User unavailable": "Usuario no disponible",
  "Usual service hint": "Pista de servicio habitual",
  "Verified entries": "Entradas verificadas",
  "Verified read-only bundle with {count} connections.":
    "Paquete de solo lectura verificado con {count} conexiones.",
  "Verify and open read-only": "Verificar y abrir en solo lectura",
  "Virtual 100k+": "Virtual 100k+",
  "Virtualized connection evidence": "Evidencia de conexiones virtualizada",
  Visible: "Visible",
  "VPN interface observed": "Interfaz VPN observada",
  "What changed today": "Qué cambió hoy",
  "What is listening, since when, and where its bind may be reachable. No network scan is performed.":
    "Qué está escuchando, desde cuándo y dónde podría alcanzar su bind. No se realiza ningún escaneo de red.",
  "Wildcard means all local interfaces; it does not prove Internet accessibility.":
    "Wildcard significa todas las interfaces locales; no demuestra accesibilidad desde Internet.",
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
  PORT: "PUERTO",
  Process: "Proceso",
  Connections: "Conexiones",
  Countries: "Países",
  Traffic: "Tráfico",
  unknown: "desconocido",
  destinations: "destinos",
  Destination: "Destino",
  Network: "Red",
  Unknown: "Desconocido",
  active: "activas",
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
  "Axvaris has no analytics SDK, account, subscription, remote logging or paid API. Reverse DNS is opt-in; GeoIP lookup uses an optional local free database.":
    "Axvaris no tiene analítica, cuenta, suscripción, registro remoto ni API de pago. El DNS inverso es opcional y GeoIP usa una base de datos local gratuita.",
  "Optional local DB-IP Lite data. Location is approximate and never authoritative.":
    "Datos locales opcionales de DB-IP Lite. La ubicación es aproximada y nunca autoritativa.",
  "GeoIP database": "Base de datos GeoIP",
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
  "Interactive globe showing filtered network destinations. Use the connection feed for a keyboard-accessible alternative.":
    "Globo interactivo con destinos de red filtrados. Usa el feed de conexiones como alternativa accesible mediante teclado.",
  "Loading local globe...": "Cargando globo local...",
  "Loading detailed local map...": "Cargando mapa local detallado...",
  "Loading live workspace...": "Cargando espacio de trabajo en vivo...",
  "Loading local history...": "Cargando historial local...",
  "Loading local summary...": "Cargando resumen local...",
  "Loading exposure data...": "Cargando datos de exposición...",
  "Loading local settings...": "Cargando ajustes locales...",
  "Traffic samples: every {seconds} seconds; high fidelity preserves one-second detail.":
    "Muestras de tráfico: cada {seconds} segundos; la alta fidelidad conserva detalle de un segundo.",
  "Loading next page...": "Cargando página siguiente...",
  "Load 500 more records": "Cargar 500 registros más",
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
  "Capture stopped": "Captura detenida",
  "Starting local observability engine...":
    "Iniciando el motor local de observabilidad...",
  "Command palette": "Paleta de comandos",
  "Type a command...": "Escribe un comando...",
  "Switch to 3D globe": "Cambiar al globo 3D",
  "Switch to 2D map": "Cambiar al mapa 2D",
  "Search process": "Buscar proceso",
  "Search destination": "Buscar destino",
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
  "MMDB kind": "Tipo de MMDB",
  "MMDB file path": "Ruta del archivo MMDB",
  "Verify and import": "Verificar e importar",
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
  "Local storage is unavailable. Axvaris has kept the original diagnostic below.":
    "El almacenamiento local no está disponible. Axvaris conserva debajo el diagnóstico original.",
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
  "{changed} of {total} derived rows changed":
    "Cambiaron {changed} de {total} filas derivadas",
  "{count} estimated connections to {destination}":
    "{count} conexiones estimadas con {destination}",
  "{count} pinned connections": "{count} conexiones fijadas",
  "A cancelable local job creates a derived view. Original IPs, timestamps, evidence and observed enrichment are never overwritten.":
    "Un trabajo local cancelable crea una vista derivada. Las IP, marcas temporales, evidencias y enriquecimiento observados nunca se sobrescriben.",
  "Accuracy radius": "Radio de precisión",
  "Address scope": "Ámbito de dirección",
  "All local data and settings": "Todos los datos y ajustes locales",
  "Application logs": "Registros de la aplicación",
  APPROXIMATE: "APROXIMADOS",
  "Approximate local location": "Ubicación local aproximada",
  "At capture": "Al capturar",
  Built: "Compilada",
  "Cancel job": "Cancelar trabajo",
  "Capability coverage": "Cobertura de capacidades",
  "Capture generation": "Generación de captura",
  "Check and compact SQLite": "Comprobar y compactar SQLite",
  "Check free City update": "Buscar actualización City gratuita",
  "Clear pins": "Quitar fijaciones",
  close: "cierre",
  "Collapse feed": "Contraer feed",
  Comfortable: "Cómodo",
  Compact: "Compacto",
  "Compact fits more evidence on screen; Comfortable adds breathing room without hiding fields.":
    "Compacto muestra más evidencia en pantalla; Cómodo añade espacio sin ocultar campos.",
  "Compare captured location with the current database":
    "Comparar la ubicación capturada con la base actual",
  "Copy redacted report": "Copiar informe anonimizado",
  CORRELATED: "CORRELACIONADOS",
  Correlation: "Correlación",
  "Counter source": "Origen de contadores",
  "Current database": "Base actual",
  "Database {size} · WAL {log}/{checkpointed} pages":
    "Base {size} · WAL {log}/{checkpointed} páginas",
  Dataset: "Conjunto de datos",
  "Dataset date": "Fecha del conjunto",
  Delete: "Eliminar",
  "Delete investigation {name}": "Eliminar investigación {name}",
  "Delete one local category at a time. Capture must be stopped before changing retained sessions or database files.":
    "Elimina una categoría local cada vez. La captura debe estar detenida antes de cambiar sesiones conservadas o archivos de base de datos.",
  "Delete selected data": "Eliminar datos seleccionados",
  "Delete this saved local investigation?":
    "¿Eliminar esta investigación local guardada?",
  Diagnostic: "Diagnóstico",
  "DNS cache": "Caché DNS",
  "Downloads happen only after this action and fetch the complete free dataset over HTTPS. Observed IPs are never sent.":
    "Las descargas solo ocurren tras esta acción y obtienen el conjunto gratuito completo mediante HTTPS. Las IP observadas nunca se envían.",
  dropped: "descartados",
  Edition: "Edición",
  endpoint: "extremo",
  "Endpoint role": "Rol del extremo",
  "Ephemeral capture": "Captura efímera",
  "Estimated network location": "Ubicación de red estimada",
  Evidence: "Evidencia",
  "Evidence sequence": "Secuencia de evidencia",
  "Expand map": "Ampliar mapa",
  "Flow generation": "Generación de flujo",
  "Frame selection": "Encuadrar selección",
  Generation: "Generación",
  "GeoIP databases": "Bases de datos GeoIP",
  "Globe controls": "Controles del globo",
  Identity: "Identidad",
  "Install free City database": "Instalar base City gratuita",
  interface: "interfaz",
  "Interface density": "Densidad de interfaz",
  "Investigation name": "Nombre de la investigación",
  "IPC GAPS / RESYNCS": "HUECOS IPC / RESINCRONIZACIONES",
  "IPC REQUESTS / EMPTY": "PETICIONES IPC / VACÍAS",
  "IPC BYTES / OVERLAPS": "BYTES IPC / SOLAPAMIENTOS",
  "Last operation": "Última operación",
  "Local data category": "Categoría de datos locales",
  "Local data controls": "Controles de datos locales",
  "Local investigation workspaces": "Espacios de investigación locales",
  "Local notes": "Notas locales",
  "Map controls": "Controles del mapa",
  "Matched prefix": "Prefijo coincidente",
  "New live sessions stay in memory only: no connection history, baselines or session metadata are written to SQLite. This takes effect on the next capture.":
    "Las nuevas sesiones en vivo permanecen solo en memoria: no se escribe en SQLite historial de conexiones, referencias ni metadatos de sesión. Se aplica en la próxima captura.",
  "No active filter": "Sin filtro activo",
  "No ASN": "Sin ASN",
  "No local match": "Sin coincidencia local",
  "No saved investigations yet.": "Todavía no hay investigaciones guardadas.",
  "NON-DESTRUCTIVE GEOIP": "GEOIP NO DESTRUCTIVO",
  "Not applicable": "No aplicable",
  "Not exposed": "No expuesto",
  "Not installed": "No instalada",
  "Not provided": "No proporcionado",
  "Observed family": "Familia observada",
  "Observed IPs": "IP observadas",
  "Observed versus current GeoIP": "GeoIP observado frente al actual",
  Offline: "Sin conexión",
  "Open exact view": "Abrir vista exacta",
  ORPHANED: "HUÉRFANOS",
  "OS source": "Fuente del SO",
  "Permanently delete the selected local data? This cannot be undone inside Axvaris.":
    "¿Eliminar permanentemente los datos locales seleccionados? No se puede deshacer dentro de Axvaris.",
  "Pin visible evidence": "Fijar evidencia visible",
  pins: "fijaciones",
  Placement: "Colocación",
  Precision: "Precisión",
  "Privacy redaction": "Anonimización de privacidad",
  process: "proceso",
  "Process instance": "Instancia de proceso",
  "Re-enrich selected session": "Reenriquecer sesión seleccionada",
  Ready: "Lista",
  Reason: "Motivo",
  "Redact local IP addresses": "Anonimizar direcciones IP locales",
  "Redact user names in paths": "Anonimizar nombres de usuario en rutas",
  "Redacted self-test report": "Informe anonimizado de autoprueba",
  "Remove database": "Eliminar base de datos",
  "Remove executable paths": "Eliminar rutas de ejecutables",
  "Remove this local MMDB and its backup?":
    "¿Eliminar esta MMDB local y su copia de seguridad?",
  "Removed {count} local items from {category}.":
    "Se eliminaron {count} elementos locales de {category}.",
  "Rendering is capped at 120 routes with local geometry only.":
    "El renderizado se limita a 120 rutas usando solo geometría local.",
  Reset: "Restablecer",
  "Reset camera": "Restablecer cámara",
  "Restore map": "Restaurar mapa",
  "Retained sessions": "Sesiones conservadas",
  Rollback: "Revertir",
  "Save investigation": "Guardar investigación",
  "Save the active query, exact time range, session, pinned evidence and notes on this device. Nothing is synced.":
    "Guarda en este dispositivo la consulta activa, el intervalo exacto, la sesión, la evidencia fijada y las notas. Nada se sincroniza.",
  "SAVED INVESTIGATIONS": "INVESTIGACIONES GUARDADAS",
  "Select one retained session to enable re-enrichment.":
    "Selecciona una sesión conservada para habilitar el reenriquecimiento.",
  "Show feed": "Mostrar feed",
  "Showing the first 100 of {count} rows.":
    "Mostrando las primeras 100 de {count} filas.",
  Source: "Fuente",
  "SQLite integrity check failed": "Falló la comprobación de integridad SQLite",
  "SQLite integrity check passed": "La comprobación de integridad SQLite pasó",
  TRAFFIC: "TRÁFICO",
  Updated: "Actualizada",
  Version: "Versión",
  "Zone / interface": "Zona / interfaz",
  "Zoom in": "Acercar",
  "Zoom out": "Alejar",
  pass: "correcto",
  warn: "aviso",
  fail: "fallo",
  healthy: "saludable",
  degraded: "degradado",
  unavailable: "no disponible",
  disabled: "desactivado",
  queued: "en cola",
  "{backend}; {depth}/{capacity} queued":
    "{backend}; {depth}/{capacity} en cola",
  running: "en curso",
  completed: "completado",
  cancelled: "cancelado",
  failed: "fallido",
  capture: "captura",
  geoip: "GeoIP",
  dns: "DNS",
  storage: "almacenamiento",
  render: "renderizado",
  backpressure: "contrapresión",
  remote_peer: "peer remoto",
  local_peer: "peer local",
  listener: "listener",
  confirmed: "confirmada",
  correlated: "correlacionada",
  approximate: "aproximada",
  windows_ip_helper: "Windows IP Helper",
  windows_etw: "Windows ETW",
  linux_inet_diag: "Linux INET_DIAG",
  local_dns: "DNS local",
  local_mmdb: "MMDB local",
  demo: "demostración",
  historical: "histórico",
  global: "global",
  private: "privada",
  loopback: "bucle local",
  link_local: "enlace local",
  multicast: "multicast",
  cgnat: "CGNAT",
  documentation: "documentación",
  benchmark: "pruebas de rendimiento",
  reserved: "reservada",
  broadcast: "broadcast",
  unspecified: "no especificada",
  country: "país",
  region: "región",
  city: "ciudad",
  coordinates: "coordenadas",
  database_coordinate: "coordenada de base de datos",
  country_centroid: "centroide de país",
  not_plotted: "no representado",
  interface_aggregate: "total de interfaz",
  etw_flow: "flujo ETW",
  incomplete_record: "registro incompleto",
  historical_record: "registro histórico",
  listener_state: "estado listener",
  missing_remote_peer: "peer remoto ausente",
  connected_global_peer: "peer global conectado",
  connected_non_global_peer: "peer no global conectado",
  orphan_etw_event: "evento ETW huérfano",
  ip_helper_etw_correlation: "correlación IP Helper + ETW",
  geo_ip: "bases GeoIP",
  dns_cache: "caché DNS",
  logs: "registros",
  all: "todos los datos",
  "Capture is starting and reconciling socket state":
    "La captura está iniciándose y conciliando el estado de sockets",
  "Wait for capture to enter the Capturing state.":
    "Espera a que la captura entre en estado Capturando.",
  "Capture is stopped": "La captura está detenida",
  "Start Demo or Live capture when observation is wanted.":
    "Inicia captura Demo o en vivo cuando quieras observar.",
  "Run the guided capture self-test and review provider permissions.":
    "Ejecuta la autoprueba guiada y revisa los permisos del proveedor.",
  "No local GeoIP dataset is installed":
    "No hay ningún conjunto GeoIP local instalado",
  "Import free local MMDB datasets from Settings.":
    "Importa conjuntos MMDB locales gratuitos desde Ajustes.",
  "Rollback, remove, or re-import the affected local MMDB dataset.":
    "Revierte, elimina o vuelve a importar el conjunto MMDB local afectado.",
  "Local DNS enrichment is active": "El enriquecimiento DNS local está activo",
  "The bounded DNS queue dropped requests":
    "La cola DNS limitada descartó solicitudes",
  "Reduce capture load or disable optional reverse DNS.":
    "Reduce la carga de captura o desactiva el DNS inverso opcional.",
  "Optional reverse DNS is disabled":
    "El DNS inverso opcional está desactivado",
  "Enable it explicitly in Settings only when hostnames are needed.":
    "Actívalo expresamente en Ajustes solo cuando necesites nombres de host.",
  "SQLite quick_check passed": "SQLite quick_check pasó",
  "SQLite integrity has not passed its latest check":
    "La integridad SQLite no superó su última comprobación",
  "Stop capture, export needed data, then run local maintenance.":
    "Detén la captura, exporta los datos necesarios y ejecuta el mantenimiento local.",
  "The UI has not reported a render sample yet":
    "La interfaz aún no ha comunicado una muestra de renderizado",
  "Use compact density, 2D map, or reduced motion for dense captures.":
    "Usa densidad compacta, el mapa 2D o movimiento reducido para capturas densas.",
  "Narrow capture scope or turn off optional DNS enrichment.":
    "Reduce el ámbito de captura o desactiva el enriquecimiento DNS opcional.",
  "Reconnect the network or select All interfaces, then retry.":
    "Vuelve a conectar la red o selecciona Todas las interfaces y reintenta.",
  "Check operating-system capture permissions and retry.":
    "Comprueba los permisos de captura del sistema operativo y reintenta.",
  "Run the test again; if it persists, review the documented capture permissions.":
    "Ejecuta de nuevo la prueba; si persiste, revisa los permisos de captura documentados.",
  "Check capture permissions and retry the guided test.":
    "Comprueba los permisos de captura y repite la prueba guiada.",
  "Enable reverse DNS in Settings only if hostname enrichment is wanted.":
    "Activa el DNS inverso en Ajustes solo si deseas enriquecer nombres de host.",
  "Import a free local City/ASN MMDB or use the explicit free dataset update.":
    "Importa una MMDB City/ASN local gratuita o usa la actualización gratuita explícita.",
  "Process instance observed": "Instancia de proceso observada",
  "Process instance was not available":
    "La instancia de proceso no estaba disponible",
  "Connection opened": "Conexión abierta",
  "Local DNS name correlated": "Nombre DNS local correlacionado",
  "Estimated network location enriched":
    "Ubicación de red estimada enriquecida",
  "Connection closed": "Conexión cerrada",
  "Derived view saved; original observations remain unchanged.":
    "Vista derivada guardada; las observaciones originales permanecen sin cambios.",
  "Unterminated quoted value": "Valor entre comillas sin terminar",
  "Use tcp or udp": "Usa tcp o udp",
  "Port must be between 0 and 65535": "El puerto debe estar entre 0 y 65535",
  "Use in, out or unknown": "Usa in, out o unknown",
  "Enter an IPv4 or IPv6 address": "Introduce una dirección IPv4 o IPv6",
  "Unknown connection state": "Estado de conexión desconocido",
  "Use true or false": "Usa true o false",
  "ASN must be a number": "El ASN debe ser un número",
  "Enter a valid IPv4 or IPv6 CIDR prefix":
    "Introduce un prefijo CIDR IPv4 o IPv6 válido",
  "Unknown address scope": "Ámbito de dirección desconocido",
  "Unknown evidence source": "Fuente de evidencia desconocida",
  "Missing value after negation": "Falta un valor después de la negación",
  "The status of the local GeoIP datasets could not be read.":
    "No se pudo leer el estado de los conjuntos GeoIP locales.",
  "The previous local GeoIP dataset could not be restored; the active dataset was kept.":
    "No se pudo restaurar el conjunto GeoIP local anterior; se conservó el conjunto activo.",
  "The selected local GeoIP dataset could not be removed safely.":
    "No se pudo eliminar de forma segura el conjunto GeoIP local seleccionado.",
  "SQLite maintenance could not complete; the database was left intact.":
    "El mantenimiento SQLite no pudo completarse; la base quedó intacta.",
  "Stop capture before running SQLite maintenance.":
    "Detén la captura antes de ejecutar el mantenimiento SQLite.",
  "Stop capture before deleting retained sessions or all local data.":
    "Detén la captura antes de eliminar sesiones conservadas o todos los datos locales.",
  "The selected local data could not be removed completely.":
    "No se pudieron eliminar por completo los datos locales seleccionados.",
  "The local investigation is missing a valid name or query.":
    "A la investigación local le falta un nombre o una consulta válidos.",
  "Saved investigations could not be read from local storage.":
    "No se pudieron leer las investigaciones guardadas del almacenamiento local.",
  "Choose a valid ASN number greater than zero.":
    "Elige un número ASN válido mayor que cero.",
  "The local ASN profile could not be reconstructed from retained evidence.":
    "No se pudo reconstruir el perfil ASN local a partir de la evidencia conservada.",
  "The selected saved connection is no longer available for its identity timeline.":
    "La conexión guardada seleccionada ya no está disponible para su línea temporal de identidad.",
  "Select one retained session to re-enrich.":
    "Selecciona una sesión conservada para reenriquecerla.",
  "This session already has an active re-enrichment job.":
    "Esta sesión ya tiene un trabajo de reenriquecimiento activo.",
  "The local re-enrichment job is no longer available.":
    "El trabajo local de reenriquecimiento ya no está disponible.",
  "The derived GeoIP view could not be loaded; original observations were unchanged.":
    "No se pudo cargar la vista GeoIP derivada; las observaciones originales no cambiaron.",
  "Choose two different retained sessions.":
    "Elige dos sesiones conservadas diferentes.",
  "The local session comparison could not be exported.":
    "No se pudo exportar la comparación local de sesiones.",
  "Packet recording could not continue; review the original local diagnostic.":
    "La grabación de paquetes no pudo continuar; revisa el diagnóstico local original.",
  "Local focus rules could not be read.":
    "No se pudieron leer las reglas de enfoque locales.",
  "The local focus rule is invalid.": "La regla de enfoque local no es válida.",
  "The local history query is invalid.":
    "La consulta de historial local no es válida.",
  "TCP/UDP and IPv4/IPv6 capability proof":
    "Prueba de capacidades TCP/UDP e IPv4/IPv6",
  "Review the capability rows; retry with the documented permissions or enable the missing local IP stack.":
    "Revisa las filas de capacidades; reintenta con los permisos documentados o habilita la pila IP local ausente.",
  "The current Axvaris process was resolved from controlled loopback sockets.":
    "El proceso actual de Axvaris se identificó mediante sockets de bucle local controlados.",
  "The native provider did not expose the controlled local endpoint":
    "El proveedor nativo no expuso el extremo local controlado",
  "The operating system did not expose the proof process identity":
    "El sistema operativo no expuso la identidad del proceso de prueba",
  "The provider did not expose an interface for this loopback row":
    "El proveedor no expuso una interfaz para esta fila de bucle local",
  "The close was not visible within the bounded 30 ms proof window":
    "El cierre no fue visible dentro de la ventana acotada de 30 ms",
  "Bound UDP rows stay local/listener evidence when the OS omits their peer":
    "Las filas UDP enlazadas permanecen como evidencia local/listener cuando el SO omite su peer",
  "Application and network profiles": "Perfiles de aplicación y red",
  "Application profile": "Perfil de aplicación",
  "binary revisions": "revisiones del binario",
  "Both profiles have the same observed VPN flag.":
    "Ambos perfiles tienen el mismo indicador de VPN observado.",
  "Cancel hashing": "Cancelar cálculo del hash",
  "Connections by local hour": "Conexiones por hora local",
  "Created {date}": "Creado el {date}",
  "Current v5 history does not retroactively assign a network profile to legacy connection rows. Comparisons remain scoped to captured profile evidence and never guess an association.":
    "El historial v5 actual no asigna retroactivamente un perfil de red a conexiones antiguas. Las comparaciones se limitan a la evidencia de perfil capturada y nunca inventan una asociación.",
  "Direct network observed": "Red directa observada",
  domains: "dominios",
  Domains: "Dominios",
  "Earlier network profile": "Perfil de red anterior",
  End: "Fin",
  "Every aggregate below is computed from the exact common query and drills back to retained connection IDs.":
    "Cada agregado se calcula a partir de la consulta común exacta y permite llegar hasta los identificadores de conexión conservados.",
  "Executable revisions": "Revisiones del ejecutable",
  "File revision": "Revisión del archivo",
  "First observed": "Primera observación",
  "Format / package": "Formato / paquete",
  "Hash and signature are local identity evidence, never reputation or a safety verdict.":
    "El hash y la firma son evidencia local de identidad, nunca reputación ni un veredicto de seguridad.",
  "HISTORICAL CONTEXT": "CONTEXTO HISTÓRICO",
  "Inspect executable locally": "Inspeccionar ejecutable localmente",
  "Last observed": "Última observación",
  "Later network profile": "Perfil de red posterior",
  listeners: "listeners",
  "Local name for {id}": "Nombre local para {id}",
  "Namespace / cgroup": "Espacio de nombres / cgroup",
  "No evidence in the selected window.":
    "No hay evidencia en la ventana seleccionada.",
  "Not computed": "No calculado",
  "Not inspected yet": "Aún no inspeccionado",
  "Observed paths": "Rutas observadas",
  "observed traffic": "tráfico observado",
  "Parent instance": "Instancia padre",
  "Per-user startup registration could not be changed. The previous setting was preserved.":
    "No se pudo cambiar el inicio automático por usuario. Se conservó el ajuste anterior.",
  "Ports and protocols": "Puertos y protocolos",
  "Pseudonymous network profiles": "Perfiles de red seudónimos",
  "Quiet hours": "Horas de silencio",
  "Raw SSID, gateway and addresses are discarded from the profile identity; names are local and optional.":
    "El SSID, la puerta de enlace y las direcciones sin redactar se descartan de la identidad del perfil; los nombres son locales y opcionales.",
  "Signature status": "Estado de la firma",
  Start: "Inicio",
  "Suppresses new local alerts during the selected local-time window; grouped evidence remains retained.":
    "Suprime nuevos avisos locales durante la franja horaria seleccionada; la evidencia agrupada se conserva.",
  "The observed VPN flag changed between these profiles.":
    "El indicador de VPN observado cambió entre estos perfiles.",
  "User / session": "Usuario / sesión",
  "Usual hours": "Horas habituales",
  "VPN observed": "VPN observada",
  "{count} in this category.": "{count} en esta categoría.",
  "{count} retained changes": "{count} cambios conservados",
  "A capture or enrichment capability is degraded.":
    "Una capacidad de captura o enriquecimiento está degradada.",
  "A comparison appears after a retained session contains connection evidence.":
    "La comparación aparecerá cuando una sesión conservada contenga evidencia de conexiones.",
  "A listener changed to a broader bind scope.":
    "Un listener cambió a un alcance de escucha más amplio.",
  "A listening endpoint from the previous snapshot is no longer present.":
    "Un endpoint en escucha del snapshot anterior ya no está presente.",
  "A local listening endpoint appeared; review bind scope and firewall evidence.":
    "Apareció un endpoint local en escucha; revisa el alcance y la evidencia del firewall.",
  "Active connections": "Conexiones activas",
  "Active processes": "Procesos activos",
  Added: "Añadidos",
  "Aggregate counters": "Contadores agregados",
  "Applies from 90% to 200% and is clamped to a safe migrated value.":
    "Se aplica del 90 % al 200 % y se limita a un valor migrado seguro.",
  "Capture active": "Captura activa",
  "Capture mode in compact menu": "Modo de captura en el menú compacto",
  "Capture status: {status}": "Estado de la captura: {status}",
  "Compared with {date}": "Comparado con {date}",
  "Configure local GeoIP": "Configurar GeoIP local",
  "Counters unavailable": "Contadores no disponibles",
  "CURRENT SESSION": "SESIÓN ACTUAL",
  "Deterministic demo counters": "Contadores deterministas de la demo",
  "Disable presentation mode": "Desactivar el modo presentación",
  "Enable presentation mode": "Activar el modo presentación",
  "Enrichment pending": "Enriquecimiento pendiente",
  "Every current target has a usable plotted position.":
    "Todos los destinos actuales tienen una posición utilizable.",
  "Filter unlocated evidence": "Filtrar evidencia sin ubicación",
  "First observed country estimate in retained evidence; location is approximate.":
    "Primera estimación de país observada en la evidencia conservada; la ubicación es aproximada.",
  "First observed domain in the complete retained local baseline.":
    "Primer dominio observado en toda la referencia local conservada.",
  "GeoIP data absent or no local match":
    "Datos GeoIP ausentes o sin coincidencia local",
  Inspect: "Inspeccionar",
  "Inspect {process} to {destination}: {reason}":
    "Inspeccionar {process} hacia {destination}: {reason}",
  "Listener added": "Listener añadido",
  "Listener removed": "Listener eliminado",
  "Listener scope widened": "Alcance del listener ampliado",
  "Local or reserved destination": "Destino local o reservado",
  "Location precision is too low":
    "La precisión de ubicación es demasiado baja",
  "Missing coordinates stay unplotted; Axvaris never invents a location.":
    "Los destinos sin coordenadas no se representan; Axvaris nunca inventa una ubicación.",
  "Network interface in compact menu": "Interfaz de red en el menú compacto",
  "Network profile change": "Cambio de perfil de red",
  "New country": "País nuevo",
  "New destination for application": "Destino nuevo para la aplicación",
  "New executable revision": "Revisión nueva del ejecutable",
  "No matching settings.": "Ningún ajuste coincide.",
  "No previous valid session": "No hay una sesión anterior válida",
  "No structural differences from the previous session.":
    "No hay diferencias estructurales respecto a la sesión anterior.",
  "No usable coordinates": "No hay coordenadas utilizables",
  "Open capture and actions menu": "Abrir el menú de captura y acciones",
  "Open presentation settings": "Abrir los ajustes de presentación",
  "Per-flow counters": "Contadores por flujo",
  Plotted: "Representados",
  Presentation: "Presentación",
  "Presentation mode": "Modo presentación",
  "Presentation mode · visual pseudonyms active":
    "Modo presentación · seudónimos visuales activos",
  "Previous session": "Sesión anterior",
  "Primary navigation in compact menu":
    "Navegación principal en el menú compacto",
  "Pseudonymizes sensitive values only in the current visual session. Stored evidence, filters and export redaction remain unchanged.":
    "Seudonimiza los valores sensibles solo en la sesión visual actual. La evidencia guardada, los filtros y la redacción de las exportaciones no cambian.",
  Received: "Recibido",
  "Received difference": "Diferencia recibida",
  Removed: "Eliminados",
  "Remove ignore rule {value}": "Eliminar la regla de exclusión {value}",
  "Search settings": "Buscar ajustes",
  "Search settings in English or Spanish": "Buscar ajustes en inglés o español",
  Sent: "Enviado",
  "Sent difference": "Diferencia enviada",
  "The pseudonymous local network profile changed during this session.":
    "El perfil de red local seudónimo cambió durante esta sesión.",
  "This application revision was not present in retained network evidence.":
    "Esta revisión de la aplicación no estaba presente en la evidencia de red conservada.",
  "Traffic totals are not comparable because counter sources differ or are unavailable. Structural changes remain valid.":
    "Los totales de tráfico no son comparables porque las fuentes de los contadores difieren o no están disponibles. Los cambios estructurales siguen siendo válidos.",
  "Traffic trend unavailable": "Tendencia de tráfico no disponible",
  "Traffic trend with {count} samples; latest {rate}":
    "Tendencia de tráfico con {count} muestras; última tasa: {rate}",
  Unlocated: "Sin ubicación",
  "Unlocated target evidence": "Evidencia de destinos sin ubicación",
  "Visual pseudonyms are active": "Los seudónimos visuales están activos",
  Widened: "Ampliados",
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
  const controlledSocketRows = source.match(
    /^(\d+) TCP\/UDP rows observed during controlled local loopback proofs$/,
  );
  if (controlledSocketRows) {
    return `${controlledSocketRows[1]} filas TCP/UDP observadas durante pruebas de bucle local controladas`;
  }
  const exposedCapabilities = source.match(
    /^(\d+) of (\d+) local protocol\/family fixtures exposed their endpoints$/,
  );
  if (exposedCapabilities) {
    return `${exposedCapabilities[1]} de ${exposedCapabilities[2]} fixtures locales de protocolo/familia expusieron sus extremos`;
  }
  const fixtureUnavailable = source.match(
    /^Local loopback fixture unavailable: (.+)$/,
  );
  if (fixtureUnavailable) {
    return `Fixture de bucle local no disponible: ${fixtureUnavailable[1]}`;
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
  const unknownFilter = source.match(/^Unknown filter: (.+)$/);
  if (unknownFilter) return `Filtro desconocido: ${unknownFilter[1]}`;
  const missingFilterValue = source.match(/^Missing value for (.+)$/);
  if (missingFilterValue) return `Falta el valor de ${missingFilterValue[1]}`;
  const invalidFilterValue = source.match(/^([^:]+): (.+)$/);
  const invalidFilterReason = invalidFilterValue?.[2];
  if (
    invalidFilterValue &&
    invalidFilterReason &&
    spanish[invalidFilterReason]
  ) {
    return `${invalidFilterValue[1]}: ${spanish[invalidFilterReason]}`;
  }
  const capturing = source.match(/^(.+) is capturing$/);
  if (capturing) return `${capturing[1]} está capturando`;
  const geoGenerations = source.match(
    /^City generation (\d+), ASN generation (\d+)$/,
  );
  if (geoGenerations) {
    return `Generación City ${geoGenerations[1]}, generación ASN ${geoGenerations[2]}`;
  }
  const renderRate = source.match(/^Render rate is (\d+) FPS$/);
  if (renderRate) return `La tasa de renderizado es ${renderRate[1]} FPS`;
  const queuePressure = source.match(
    /^Queue utilization (\d+)%; (\d+) enrichment jobs dropped$/,
  );
  if (queuePressure) {
    return `Uso de cola ${queuePressure[1]} %; ${queuePressure[2]} trabajos de enriquecimiento descartados`;
  }
  const queueSlots = source.match(/^(\d+) of (\d+) bounded queue slots used$/);
  if (queueSlots) {
    return `${queueSlots[1]} de ${queueSlots[2]} espacios de cola limitada usados`;
  }
  const finalState = source.match(/^Final state: (.+)$/);
  if (finalState) return `Estado final: ${finalState[1]}`;
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
