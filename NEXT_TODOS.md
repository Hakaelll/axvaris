# Sabicheiro: próximas mejoras significativas

Última actualización: **2026-08-12**
Versión objetivo: **0.3.0 — Verified Destinations**

Este documento conserva la hoja de ruta y su cierre verificable. La implementación de
la 0.3.0 está terminada y se generó un release local de prueba para Windows. La evidencia
completa está en `docs/VALIDATION.md`: 39 tareas están `DONE` y ocho comprobaciones que
requieren Linux, elevación aprobada, lector de pantalla del sistema o instalación en un
entorno desechable permanecen honestamente en `REVIEW`. No se añadió ninguna API de
pago, cuenta, telemetría ni servicio alojado.

## Resultado esperado

Al terminar esta hoja de ruta, Sabicheiro debe:

1. identificar y normalizar correctamente cada IP observada antes de enriquecerla;
2. distinguir un peer remoto real de un listener, una dirección local o un registro
   incompleto del sistema operativo;
3. localizar IP públicas con una base GeoIP local y mostrar la procedencia, antigüedad
   y precisión real del dato;
4. permitir investigar la relación
   `proceso → conexión → IP → dominio → ASN → ubicación → historial`;
5. ofrecer un globo 3D más expresivo, puntos de ubicación legibles y un mapa 2D
   interactivo que pueda ampliarse sin perder selección ni contexto;
6. conservar únicamente código, recursos y configuraciones utilizados por las
   aplicaciones de escritorio para Windows y Linux;
7. superar pruebas nativas, visuales, de rendimiento, privacidad y empaquetado antes
   de publicar la release.

## Reglas del plan

### Estados

| Estado | Significado |
| --- | --- |
| `TODO` | No iniciado |
| `IN_PROGRESS` | Implementación activa; todavía no cumple todos los criterios |
| `BLOCKED` | Existe un bloqueo externo documentado con una acción concreta |
| `REVIEW` | Implementado; falta una validación nativa, visual o de privilegios |
| `DONE` | Código, pruebas, documentación y evidencia de aceptación completos |

### Prioridades

| Prioridad | Significado |
| --- | --- |
| `P0` | Bloquea la release 0.3.0 |
| `P1` | Mejora principal de la release; solo puede aplazarse con justificación |
| `P2` | Mejora posterior que no debe retrasar un núcleo ya validado |

No se marcará una tarea como `DONE` solo porque compile. Cada tarea debe incluir
pruebas proporcionales al riesgo, textos en inglés y castellano cuando afecte a la UI,
documentación actualizada y evidencia en `docs/VALIDATION.md`.

## Qué significa detectar y localizar correctamente una IP

Una IP se considera **detectada correctamente** únicamente cuando:

- procede de una fuente nativa identificada: IP Helper, ETW, Netlink `INET_DIAG` o
  Demo explícito;
- se ha convertido a un `IpAddr` tipado y a una representación canónica;
- conserva protocolo, endpoints, PID/instancia de proceso, interfaz, instante y fuente;
- el rol del endpoint está justificado como remoto, local, listener o desconocido;
- no se ha fabricado un peer para sockets UDP sin conexión ni para listeners comodín;
- las observaciones duplicadas de snapshot y eventos se correlacionan sin mezclar flujos;
- las limitaciones de permisos o del sistema se muestran como tales.

Una IP se considera **localizada correctamente** únicamente cuando:

- es una dirección global apta para GeoIP; las redes privadas, loopback, link-local,
  multicast, CGNAT, documentación, benchmark, reservadas y no especificadas se
  clasifican localmente y nunca se envían al localizador;
- la respuesta proviene de una MMDB local validada y queda asociada a su edición,
  fecha de compilación, hash y generación;
- país, subdivisión, ciudad, coordenadas, ASN y organización pueden faltar de forma
  independiente sin invalidar los demás campos;
- se distingue entre ubicación de ciudad, centroide de país y dato desconocido;
- la interfaz la describe como una estimación de red, nunca como la posición física
  exacta de una persona o dispositivo;
- una IP sin coincidencia permanece visible como IP desconocida y no se dibuja en
  coordenadas inventadas.

DNS y GeoIP son evidencias diferentes: un nombre resuelto no determina por sí mismo
la ubicación, y una ubicación no demuestra la identidad del propietario del dominio.

## Línea A — Fidelidad de endpoints y captura

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| IP-01 | Modelo canónico de dirección y alcance | P0 | DONE | — |
| IP-02 | Rol y dirección del peer basados en evidencia | P0 | DONE | IP-01 |
| IP-03 | Identidad estable del flujo y del proceso | P0 | DONE | IP-01 |
| IP-04 | Correlación Windows IP Helper + ETW | P0 | REVIEW | IP-02, IP-03 |
| IP-05 | Paridad Linux con Netlink `INET_DIAG` | P0 | REVIEW | IP-02, IP-03 |
| IP-06 | Persistencia de procedencia y migración v4 | P0 | DONE | IP-01, IP-03 |
| IP-07 | Autodiagnóstico de exactitud de captura | P1 | DONE | IP-04, IP-05 |

### IP-01 — Modelo canónico de dirección y alcance

- Ampliar `AddressScope` para distinguir al menos global, privada, loopback,
  link-local, multicast, CGNAT, documentación, benchmark, reservada, broadcast y no
  especificada en IPv4 e IPv6.
- Canonicalizar IPv4-mapped IPv6 y formatos textuales equivalentes sin perder la familia
  observada cuando sea relevante para el diagnóstico.
- Mantener el identificador de interfaz o zona separado de la IP.
- Añadir pruebas de límites para cada prefijo especial y pruebas basadas en propiedades
  de parseo/formateo.

**Aceptación:** una tabla versionada de casos IPv4/IPv6 cubre los límites de cada rango;
ninguna dirección no global llega al resolver GeoIP.

### IP-02 — Rol y dirección del peer basados en evidencia

- Modelar `EndpointRole` como `remote_peer`, `local_peer`, `listener` o
  `unknown`, junto con la razón y fuente que lo determinó.
- No inferir salida/entrada únicamente comparando una IP pública con una privada.
- Separar listeners TCP, sockets UDP enlazados y conexiones UDP con peer conocido.
- Conservar filas incompletas como diagnóstico sin mezclarlas con destinos remotos.

**Aceptación:** fixtures de TCP/UDP, IPv4/IPv6, loopback, LAN, VPN y wildcard producen
el workspace y la dirección esperados sin destinos falsos.

### IP-03 — Identidad estable del flujo y del proceso

- Reemplazar la identidad basada solo en PID + 5-tupla por una identidad que incluya
  instancia de proceso, protocolo, interfaz y generación temporal.
- Detectar reutilización de PID, reapertura rápida de puertos y reinicios de captura.
- Definir reglas deterministas de apertura, actualización, cierre y expiración visual.

**Aceptación:** las pruebas reproducen reutilización de PID, reconexiones y flujos
simultáneos sin fusionar historial ni contadores.

### IP-04 — Correlación Windows IP Helper + ETW

- Correlacionar eventos y snapshots mediante 5-tupla, PID, dirección, interfaz y una
  ventana temporal acotada.
- Deduplicar observaciones sin perder bytes, paquetes o transiciones de estado.
- Registrar calidad de correlación, eventos huérfanos, pérdidas ETW y fallback activo.
- Añadir una prueba nativa controlada con servidores TCP/UDP locales y un endpoint
  externo de prueba configurable; la suite ordinaria no dependerá de Internet.

**Aceptación:** en Windows normal y elevado, el informe distingue claramente datos
confirmados, aproximados y no disponibles; nunca reparte tráfico agregado entre flujos.

### IP-05 — Paridad Linux con Netlink `INET_DIAG`

- Validar TCP/UDP IPv4/IPv6, network namespaces visibles, inode → proceso y sockets que
  desaparecen durante el muestreo.
- Documentar de forma exacta qué campos requieren permisos o capacidades.
- Mantener `/proc/<pid>/fd` únicamente para atribución de propietario.

**Aceptación:** integración nativa en Ubuntu/Debian compara sockets creados por una
fixture con los eventos normalizados esperados y no usa `/proc/net`.

### IP-06 — Persistencia de procedencia y migración v4

- Guardar alcance, rol del endpoint, fuente de observación, calidad de correlación,
  instancia de proceso y generación GeoIP.
- Migrar bases v1–v3 de forma transaccional y reversible mediante copia de seguridad.
- Mantener compatibilidad de lectura para sesiones antiguas sin inventar campos.

**Aceptación:** migraciones desde cada versión soportada y rollback ante fallo pasan en
SQLite; una sesión reconstruida explica de dónde procede cada dato.

### IP-07 — Autodiagnóstico de exactitud de captura

- Añadir una prueba guiada que abra sockets locales controlados y compruebe interfaz,
  PID, endpoints, dirección y cierre.
- Mostrar cobertura por protocolo/familia y limitaciones activas del proveedor.
- Permitir copiar un informe redactado sin incluir IP históricas ni payload.

**Aceptación:** el panel ofrece pass/warn/fail por capacidad y una acción concreta para
cada warning.

## Línea B — GeoIP local fiable y explicable

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| GEO-01 | Modelo de procedencia y precisión geográfica | P0 | DONE | IP-01 |
| GEO-02 | Ciclo de vida seguro de City/ASN MMDB | P0 | DONE | GEO-01 |
| GEO-03 | Enriquecimiento asíncrono y caché por generación | P0 | DONE | GEO-02 |
| GEO-04 | Re-enriquecimiento histórico no destructivo | P1 | DONE | IP-06, GEO-03 |
| GEO-05 | Presentación honesta de ubicación | P0 | DONE | GEO-01, VIS-04 |
| GEO-06 | Perfil de ASN y red de destino | P1 | DONE | GEO-01 |
| GEO-07 | Actualización, rollback y trabajo offline | P0 | DONE | GEO-02 |
| GEO-08 | Suite determinista de exactitud GeoIP | P0 | DONE | GEO-01, GEO-03 |

### GEO-01 — Modelo de procedencia y precisión

- Añadir `GeoPrecision` (`country`, `region`, `city`, `coordinates`,
  `unknown`) y `PlacementKind` (`database_coordinate`,
  `country_centroid`, `not_plotted`).
- Guardar fuente, tipo/edición de base, build epoch, hash, prefijo coincidente y
  generación de datos.
- Incorporar radio de precisión cuando exista; nunca inventarlo si la MMDB no lo ofrece.
- Validar códigos ISO, números finitos y rangos de latitud/longitud.

**Aceptación:** toda ubicación visible permite abrir “Fuente y precisión”; valores
parciales o inválidos degradan a desconocido sin romper la conexión.

### GEO-02 — Ciclo de vida seguro de City/ASN MMDB

- Tratar City y ASN como datasets independientes con estados separados.
- Validar tamaño, formato, edición, estructura mínima, fecha, hash y capacidad de abrir
  una consulta antes de activar un archivo.
- Instalar en temporal, hacer swap atómico y conservar la última versión válida para
  rollback.
- Rechazar archivos que sean otra edición aunque tengan extensión `.mmdb`.

**Aceptación:** archivos correctos, truncados, sobredimensionados, de edición equivocada
y con permisos insuficientes tienen pruebas; un fallo nunca destruye la base activa.

### GEO-03 — Enriquecimiento asíncrono y caché por generación

- Sacar las consultas y recargas de la ruta crítica de captura.
- Usar una cola acotada, coalescer solicitudes repetidas y exponer profundidad/pérdidas.
- Clave de caché: IP canónica + generación de City + generación de ASN.
- Invalidar resultados negativos y positivos al cambiar la base.
- Emitir una actualización incremental para completar filas ya visibles.

**Aceptación:** una actualización de MMDB cambia resultados sin reiniciar la aplicación
y sin pausar la captura; un test de carga demuestra memoria acotada.

### GEO-04 — Re-enriquecimiento histórico no destructivo

- Conservar el enriquecimiento observado originalmente junto con su generación.
- Ofrecer recalcular una sesión con la base actual como una vista derivada, sin cambiar
  IP, timestamps ni evidencia original.
- Procesar por lotes cancelables y mostrar progreso.

**Aceptación:** cancelar o fallar no deja una sesión parcialmente sobrescrita; se puede
comparar “ubicación al capturar” con “base actual”.

### GEO-05 — Presentación honesta de ubicación

- Mostrar país/ciudad como “ubicación de red estimada” y la fecha del dataset.
- No dibujar privadas, reservadas ni desconocidas; los centroides de país deben llevar
  un estilo distinto de las coordenadas de base.
- Ofrecer “por qué aparece aquí” con alcance, precisión y fuente.
- Diferenciar claramente ubicación, hostname, dominio, ASN y organización.

**Aceptación:** el feed, inspector, mapa, globo, destinos, historial y exportaciones
representan el mismo estado; ninguna pantalla convierte `unknown` en una coordenada.

### GEO-06 — Perfil de ASN y red de destino

- Crear una ficha local de ASN con organización, IP observadas, procesos relacionados,
  volumen, primera/última observación y países estimados.
- Agrupar por ASN sin afirmar que la organización sea dueña del dominio o del contenido.
- Permitir navegar ASN ↔ IP ↔ proceso ↔ sesión.

**Aceptación:** los totales coinciden con filtros e historial y todas las relaciones
pueden rastrearse hasta conexiones concretas.

### GEO-07 — Actualización, rollback y trabajo offline

- Mostrar versión instalada, edad, próxima recomendación y último resultado.
- Actualización solo bajo acción/consentimiento del usuario, mediante HTTPS y descarga
  completa del dataset; nunca enviar IP observadas al proveedor.
- Soportar importación offline, rollback y eliminación segura desde Ajustes.
- Documentar atribución/licencia junto al dataset activo.

**Aceptación:** instalación inicial, actualización, cancelación, falta de red, disco
lleno, rollback e importación offline tienen estados recuperables y localizados.

### GEO-08 — Suite determinista de exactitud GeoIP

- Crear fixtures MMDB sintéticas versionadas para City y ASN.
- Cubrir match exacto, longest-prefix match, IPv4/IPv6, dato parcial, sin match,
  coordenadas inválidas y cambio de generación.
- Separar pruebas de clasificación de rangos de las pruebas del lector MMDB.
- No afirmar ciudades reales de IP públicas cambiantes en la suite automatizada.

**Aceptación:** las pruebas funcionan totalmente offline y producen resultados idénticos
en Windows y Linux.

## Línea C — Flujo de investigación

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| INV-01 | Vista “Destination intelligence” | P1 | DONE | GEO-05, GEO-06 |
| INV-02 | Inspector de cadena de evidencia | P1 | DONE | IP-06, GEO-05 |
| INV-03 | Línea temporal DNS/IP/proceso | P1 | DONE | IP-03 |
| INV-04 | Búsqueda por IP, CIDR, ASN y alcance | P1 | DONE | IP-01, GEO-06 |
| INV-05 | Espacios de investigación guardados | P2 | DONE | INV-01, INV-04 |
| INV-06 | Exportación con procedencia y redacción | P1 | DONE | IP-06, GEO-01 |

### Cambios funcionales

- **Destination intelligence:** ranking de destinos con IP canónica, ASN, ubicación
  estimada, procesos, puertos, volumen, recurrencia y primera/última observación.
- **Cadena de evidencia:** un inspector explica qué observó el SO, cómo se correlacionó,
  qué enriqueció DNS/GeoIP y qué campos siguen desconocidos.
- **Timeline de identidad:** muestra cuándo un proceso resolvió un nombre, cuándo abrió
  la conexión y cuándo cambió el enriquecimiento, con confianza y fuente independientes.
- **Consulta avanzada:** añadir `ip:`, `cidr:`, `asn:`, `scope:`, `domain:`,
  `geo:`, `source:` y negación, con errores de sintaxis localizados.
- **Investigaciones guardadas:** filtros, intervalo, elementos fijados y notas locales;
  no crean cuentas ni sincronización cloud.
- **Exportación verificable:** JSON/CSV incluye procedencia, precisión y versión del
  dataset; ofrece redactar IP local, ruta ejecutable y nombres de usuario.

**Aceptación común:** filtros, totales, mapa, timeline y exportación usan el mismo query
model; reconstruir una investigación produce el mismo conjunto de conexiones.

## Línea D — Rendimiento, estabilidad y privacidad

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| REL-01 | IPC incremental con secuencia y resync | P1 | DONE | IP-03 |
| REL-02 | Recuperación tras suspensión y cambio de red | P0 | DONE | IP-04, IP-05 |
| REL-03 | Backpressure y presupuestos medibles | P0 | DONE | GEO-03, REL-01 |
| REL-04 | Sesión efímera y controles de retención | P1 | DONE | IP-06 |
| REL-05 | Integridad y mantenimiento de SQLite | P1 | DONE | IP-06 |
| REL-06 | Diagnósticos accionables por subsistema | P1 | DONE | IP-07 |

- Sustituir snapshots completos frecuentes por deltas versionados; detectar huecos de
  secuencia y solicitar un snapshot de resincronización.
- Tras sleep/resume, cambio de VPN, interfaz o dirección, reiniciar baselines de
  contadores para evitar picos falsos y reconciliar flujos.
- Definir benchmarks reproducibles para 1.000, 10.000 y 50.000 conexiones sintéticas;
  establecer límites de CPU, memoria, latencia p95, tamaño IPC y pérdida antes de
  optimizar.
- Añadir modo efímero que capture sin persistir historial y acciones separadas para
  borrar sesiones, GeoIP, caché DNS, logs y todo el dato local.
- Ejecutar `quick_check`, checkpoints WAL y compactación únicamente con políticas
  seguras, cancelables y visibles.
- Aislar fallos de captura, GeoIP, DNS, almacenamiento y render; ninguno debe detener
  silenciosamente a los demás.

**Aceptación:** una prueba de estrés de 30 minutos mantiene colas y memoria acotadas; el
usuario puede ver qué subsistema degradó, desde cuándo y cómo recuperarlo.

## Línea E — Pulido visual y accesibilidad

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| VIS-01 | Jerarquía y densidad coherentes | P1 | DONE | — |
| VIS-02 | Rediseño visual e interactivo del globo 3D | P0 | DONE | GEO-01 |
| VIS-03 | Mapa 2D ampliable con zoom y paneo | P0 | DONE | GEO-01 |
| VIS-04 | Sistema compartido de puntos de ubicación | P0 | DONE | VIS-02, VIS-03 |
| VIS-05 | Inspector y perfiles rediseñados | P1 | DONE | INV-01, INV-02 |
| VIS-06 | Estados de carga, vacío, error y datos parciales | P0 | DONE | GEO-05 |
| VIS-07 | Teclado, lector de pantalla y contraste | P0 | REVIEW | VIS-01–VIS-06 |
| VIS-08 | Regresión visual multidensidad | P0 | REVIEW | VIS-01–VIS-07 |

### Estado visual que debe corregirse

La referencia actual muestra un globo funcional, pero los destinos se representan como
esferas grises planas y casi idénticas, las rutas tienen poco contraste jerárquico y el
relieve visual del planeta es demasiado tenue. El mapa 2D es un SVG fijo: no permite
acercarse, desplazarse, encuadrar una selección ni ocupar el área principal completa.
La 0.3.0 debe resolver estas limitaciones sin convertir Sabicheiro en una interfaz
decorativa o recargada.

### VIS-01 — Jerarquía y densidad coherentes

- Simplificar top bar, filtros y métricas para que la conexión seleccionada sea siempre
  el foco; eliminar adornos que no codifiquen información.
- Incorporar modos de densidad **Compacto** y **Cómodo** sin duplicar componentes.
- Hacer que mapa/globo, feed e inspector compartan selección, hover y filtros sin
  saltos de layout ni pérdida de foco.

### VIS-02 — Rediseño visual e interactivo del globo 3D

- Mantener Three.js y la cartografía local, pero reforzar profundidad mediante relleno
  de tierra, jerarquía costa/frontera, iluminación lateral suave y un halo atmosférico
  muy contenido. El fondo debe seguir la dirección “Midnight Precision Instrument”.
- Sustituir las líneas uniformes por rutas con grosor fiable en pantalla, altura basada
  en distancia geodésica y niveles claros: contexto, activa, seleccionada y cerrada.
- Añadir una indicación sutil de dirección/actividad en rutas activas; se desactiva por
  completo con `prefers-reduced-motion` o la opción de movimiento reducido.
- Aplicar oclusión y desvanecimiento correctos en el borde del planeta para que puntos
  y rutas de la cara posterior no atraviesen visualmente la esfera.
- Añadir hover con tooltip local, cursor de interacción, foco de destino, reset de
  cámara y zoom suave con límites útiles. Seleccionar una ruta no debe cambiar la cámara
  si el usuario desactivó Smart focus.
- Mantener clustering estable al rotar/acercar y evitar que una actualización de datos
  reconstruya innecesariamente toda la escena o altere la cámara.
- Renderizar con nitidez en pantallas HiDPI, sin fugas de geometrías/materiales y con un
  presupuesto explícito de rutas, puntos y draw calls.

**Aceptación:** la selección es reconocible sin depender solo del color; ningún punto
trasero aparece sobre la cara frontal; rotar, hacer zoom, enfocar y resetear conservan
selección y filtros; una captura con muchos destinos mantiene una jerarquía clara.

### VIS-03 — Mapa 2D ampliable con zoom y paneo

- Añadir un control **Ampliar mapa** en la cabecera de visualización. El modo ampliado
  ocupa toda la zona principal de la aplicación, convierte el feed en panel colapsable
  y se restaura mediante botón o `Esc`.
- Conservar al ampliar/restaurar el zoom, centro, filtros, ruta seleccionada, inspector
  y posición del feed. No remontar el mapa ni resetear el estado visual.
- Incorporar zoom con rueda/pinch, paneo por arrastre, controles `+`/`−`, **Restablecer**
  y **Encuadrar selección**; ofrecer equivalentes de teclado con foco visible.
- Definir límites de zoom/paneo y corregir rutas que cruzan el antimeridiano para evitar
  líneas que atraviesen el mapa en la dirección equivocada.
- Mejorar océano, relleno de países, fronteras, retícula y país enfocado. Mostrar nombres
  solo cuando el nivel de zoom y el espacio lo permitan, sin solapes masivos.
- Hacer que clusters se separen progresivamente al acercar y que el tooltip indique
  ubicación estimada, ASN, conexiones agrupadas y precisión.
- Mantener todos los datos y geometrías en local; no añadir tiles, trackers ni servicios
  cartográficos remotos.

**Aceptación:** el mapa funciona en tamaño normal y ampliado; zoom, paneo, encuadre,
selección, `Esc` y restauración pasan pruebas de interacción. Ningún destino válido queda
fuera de alcance por clipping y las rutas transoceánicas eligen el trazado correcto.

### VIS-04 — Puntos de ubicación más cuidados

- Reemplazar la esfera/círculo simple por un marcador compuesto: núcleo nítido, anillo
  de estado y halo muy suave. Debe verse limpio tanto sobre el globo como sobre el mapa.
- Usar una escala acotada para tamaño según conexiones agrupadas; un destino con mucho
  tráfico no puede tapar un país ni convertirse en un “blob”.
- Reservar acid-lime para origen/selección/acción primaria; usar mist/fog/graphite para
  destinos normales, inactivos e históricos. Forma, trazo y opacidad complementan el
  color para accesibilidad.
- Codificar precisión: anillo continuo para coordenada MMDB, discontinuo para centroide
  de país y estado hueco/textual para ubicación desconocida, que no se dibuja en una
  coordenada inventada.
- Mostrar un badge de cantidad para clusters y ampliar el área invisible de hit-testing
  sin agrandar el punto visual.
- Añadir un pulso breve solo al aparecer una nueva conexión; no animar continuamente
  todos los puntos y respetar movimiento reducido.
- Definir un marcador de origen diferente del destino, con tooltip “ubicación local
  aproximada” y sin presentarlo como la posición exacta del equipo.
- Compartir tokens, semántica, escala y estados entre 3D y 2D para que el mismo destino
  tenga una representación equivalente en ambas vistas.

**Aceptación:** origen, destino, cluster, selección, actividad, historial y precisión se
distinguen en tamaños normales y HiDPI; hover/click/foco tienen un área cómoda; el mapa
no depende de un exceso de glow y continúa legible en escala de grises.

### VIS-05 a VIS-07 — Inspector, estados y accesibilidad

- Rediseñar el inspector en secciones: Evidencia, Endpoint, Identidad, Ubicación,
  Tráfico, Historial y Diagnóstico.
- Unificar skeletons, progreso, vacíos, warnings y errores recuperables; ningún spinner
  puede quedar indefinidamente sin texto.
- Mantener foco visible, orden lógico, nombres accesibles, regiones live no ruidosas,
  contraste WCAG AA y reducción de movimiento.
- Completar castellano e inglés, incluyendo controles de mapa y mensajes dinámicos del
  backend; conservar el detalle técnico original en una sección expandible.
- Proporcionar en feed/inspector una alternativa semántica completa a toda interacción
  que dependa de canvas/WebGL.

### VIS-08 — Validación visual

**Aceptación:** revisión a 1280×720, 1920×1080 y 2560×1440, escalado 100/125/150 %,
mapa normal/ampliado, globo rotado/enfocado, datos vacíos/densos y
`prefers-reduced-motion`; cero scroll horizontal accidental, solapes o controles
inaccesibles. Debe completarse un recorrido con teclado y Narrator/NVDA en Windows,
además del árbol semántico automatizado. Las capturas de referencia deben incluir los
marcadores nuevos y comparar explícitamente la jerarquía de rutas con la línea base.

## Línea F — Limpieza del repositorio y alcance de plataforma

| ID | Limpieza | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| CLEAN-01 | Eliminar todo el soporte móvil no utilizado | P0 | DONE | — |
| CLEAN-02 | Reducir iconos y assets a los realmente referenciados | P0 | DONE | CLEAN-01 |
| CLEAN-03 | Eliminar código, dependencias y traducciones muertas | P1 | DONE | — |
| CLEAN-04 | Separar fuentes de archivos generados | P0 | DONE | — |
| CLEAN-05 | Consolidar documentación y configuración | P1 | DONE | CLEAN-01 |
| CLEAN-06 | Verificar build reproducible desde clon limpio | P0 | REVIEW | CLEAN-01–CLEAN-05 |

### Alcance y criterios

- Las únicas plataformas de producto son **Windows x86_64** y **Linux x86_64**.
- Eliminar carpetas, iconos, manifests, scripts, dependencias, workflows y configuración
  de **iOS y Android**: Xcode, AppIcon sets, Gradle, manifests móviles, adaptive icons,
  splash screens y targets Tauri móviles.
- No conservar Microsoft Store tiles, macOS icon sets u otros tamaños si ningún bundle
  declarado los consume. Mantener únicamente los iconos exigidos por NSIS,
  AppImage/deb y la interfaz.
- Auditar imports/exportaciones React, comandos Tauri, features Cargo, claves i18n,
  CSS, imágenes, fuentes y paquetes; eliminar elementos sin referencia después de
  confirmar que no son entradas dinámicas.
- Revisar `apps/desktop/src-tauri/gen`: versionar solo esquemas necesarios para
  desarrollo/CI o regenerarlos de manera documentada, pero no mezclar ambos modelos.
- Mantener fuera de Git `target`, `node_modules`, `dist`, stores de paquetes,
  logs, trazas, PCAP, MMDB, instaladores y releases.
- Añadir una comprobación CI que falle si reaparecen rutas móviles o artefactos
  generados prohibidos.
- Archivar el historial de tareas completadas en changelog/release notes y evitar
  duplicar documentación entre README, BUILD, arquitectura y validación.

**Aceptación:** `git ls-files` no devuelve recursos iOS/Android ni binarios generados;
la lista de assets coincide con referencias reales; un clon limpio instala, prueba y
empaqueta Windows/Linux siguiendo `BUILD.md`; `git status --short` queda vacío tras
los gates. Toda eliminación versionada es recuperable mediante Git y queda registrada
en las notas de cambio.

## Línea G — Validación y release 0.3.0

| ID | Gate | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| QA-01 | Unitarias, propiedades, fixtures y migraciones | P0 | DONE | IP, GEO |
| QA-02 | Captura nativa Windows y Linux | P0 | REVIEW | IP-04, IP-05 |
| QA-03 | Rendimiento y soak test | P0 | DONE | REL-01–REL-03 |
| QA-04 | Visual, interacción y accesibilidad | P0 | REVIEW | VIS-01–VIS-08 |
| QA-05 | Privacidad, dependencias y licencias | P0 | DONE | GEO-07, REL-04, CLEAN |
| QA-06 | Paquetes, instalación y desinstalación | P0 | REVIEW | QA-01–QA-05 |

### Gates obligatorios

1. `cargo fmt --all -- --check`.
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`.
3. `cargo test --workspace --all-targets --locked`.
4. Comprobación y build de TypeScript/Vite con lockfile.
5. Tests offline de clasificación, MMDB, correlación, migración y exportación.
6. Prueba nativa Windows normal/elevada y prueba nativa Linux sin privilegio/con las
   capacidades documentadas.
7. Benchmark y soak test con informe de baseline y límites acordados.
8. Matriz visual y recorrido completo de teclado/lector de pantalla.
9. Auditoría de privacidad: ninguna IP observada sale del dispositivo durante GeoIP;
   DNS inverso continúa siendo opt-in.
10. Auditoría de dependencias, licencias, recursos empaquetados y tamaño del bundle.
11. Smoke test de portable/NSIS en Windows y AppImage/deb en Linux, incluyendo
    actualización, desinstalación y conservación/borrado de datos según la elección.
12. Hashes SHA-256, notas de release y evidencia final en `docs/VALIDATION.md`.

## Orden de implementación

1. **Fundación:** IP-01–IP-03, GEO-01, esquema/migración IP-06.
2. **Proveedores nativos:** IP-04, IP-05 y fixtures controladas.
3. **GeoIP:** GEO-02, GEO-03, GEO-07 y GEO-08.
4. **Producto:** GEO-04–GEO-06 e INV-01–INV-06.
5. **Fiabilidad:** REL-01–REL-06.
6. **Interfaz:** VIS-01–VIS-08, incluyendo globo 3D y mapa 2D ampliable.
7. **Limpieza:** CLEAN-01–CLEAN-06 antes del freeze de release.
8. **Cierre:** QA-01–QA-06 y publicación de 0.3.0.

## Criterio de salida de la versión

La versión 0.3.0 solo puede publicarse cuando:

- no quedan tareas `P0` en `TODO`, `IN_PROGRESS`, `BLOCKED` o `REVIEW`;
- cada IP mostrada conserva endpoint, rol, fuente y calidad verificables;
- toda geolocalización visible incluye procedencia/precisión o aparece como desconocida;
- Windows y Linux superan la prueba nativa definida;
- la interfaz supera la matriz visual y de accesibilidad;
- no existen recursos iOS/Android ni artefactos no usados en el repositorio;
- el build parte de un clon limpio y deja el árbol Git limpio;
- documentación, privacidad, atribuciones y notas de release coinciden con el producto.

## Fuera de alcance

- Aplicaciones para iOS, Android o macOS.
- Telemetría, cuentas, sincronización cloud o APIs GeoIP de pago.
- Afirmar una dirección física exacta a partir de una IP.
- Inspeccionar payload por defecto, bloquear conexiones o sustituir un firewall.
- Inventar dominio, propietario, ubicación, dirección o contadores cuando el sistema no
  proporciona evidencia suficiente.
