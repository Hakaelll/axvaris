# Sabicheiro: plan de aligeramiento y mejora interna

Última actualización: **2026-08-22**

Base auditada: **0.4.2**

Objetivo: **siguiente versión de mantenimiento, centrada en peso, arranque, consumo y
mantenibilidad**

## Resultado esperado

La siguiente versión debe ofrecer las mismas funciones visibles de Sabicheiro, pero con
un ejecutable menor, un arranque más directo, menos trabajo en segundo plano y menos
consumo de CPU, memoria, GPU, IPC y disco.

Este documento sustituye la hoja de ruta anterior, que mezclaba expansión de producto,
historial de releases y validaciones ya realizadas. El historial terminado pertenece a
`CHANGELOG.md` y la evidencia de versiones publicadas a `docs/VALIDATION.md`. Hasta
cerrar este plan no se añaden grandes funciones nuevas: primero se simplifica y se mide
lo que ya existe.

## Addendum de experiencia Today y presentación — 2026-08-22

La mejora visual/arquitectónica posterior al aligeramiento está implementada y medida:

| Entrega | Estado | Evidencia cerrada |
| --- | --- | --- |
| Bootstrap Today nativo | DONE | Una llamada acotada; novedad contra todo el historial retenido; prueba fuera de las últimas 500 filas |
| Franja y comparación de sesión | DONE | Duración, procesos/conexiones, tráfico y calidad de contadores; deltas por proceso/destino/país/puerto y listeners |
| Presentation mode | DONE | Pseudónimos estables solo en memoria; texto, `title` y etiquetas accesibles; filtros, SQLite y exportaciones intactos |
| Responsive/inspector | DONE | 720×600, escala 200 %, captura siempre disponible, inspector acoplado/overlay, foco y Escape |
| Destinos sin ubicación | DONE | Cinco motivos explícitos, filtro local y acceso a GeoIP; nunca se inventan coordenadas |
| Ajustes | DONE | Búsqueda local bilingüe, foco de sección y controles de presentación/paleta |
| Robustez UI | DONE | Gates de petición obsoleta, listener estable, vistas secundarias y paneles Live diferidos |
| Estrés visual web | DONE | 50 cambios Map↔Globe, un único canvas y sin acumulación de escenas/nodos atribuible al cambio |
| Matriz nativa | REVIEW | WebView, lector de pantalla, HiDPI del SO y Linux/ETW siguen sin afirmarse sin ejecutarlos |

El artefacto de 6.528.512 B usado en la evidencia histórica no es una referencia de
paquete válida: se reproduce con `cargo build --release`, lleva `cfg(dev)` y no contiene
los chunks de `frontendDist`. El mismo `HEAD` construido con Tauri y sus 12 recursos
produce 6.857.216 B / 3.172.137 B. La entrega actual produce 6.922.240 B /
3.201.195 B: +0,95 % / +0,92 % respecto a esa referencia reproducible, dentro de los
límites de 7 MiB y 3,1 MiB. `BUILD.md` prohíbe desde ahora usar un build Cargo sin
recursos como línea base distribuible.

Al terminar:

1. La aplicación abre directamente en Today, sin el panel o mensaje `FirstRun`.
2. Live, Today, Investigate, Exposure, Data & Health, mapa 2D, globo 3D, demo,
   captura, historial, exportaciones, reglas, notificaciones, grabación, GeoIP, bandeja
   y ajustes siguen disponibles.
3. La aplicación no realiza captura ni tráfico de red por el mero hecho de abrirse.
4. No se empaquetan mapas de código fuente, recursos sobredimensionados, adaptadores de
   desarrollo ni código sin una ruta de uso real.
5. La UI deja de consultar y reconstruir estado cuando no ha cambiado o no está
   visible, sin reducir la fidelidad de la captura nativa.
6. SQLite persiste cambios reales por lotes y no reescribe todo el estado cada segundo.
7. Los ejemplos y la demo son mínimos, acotados y no contaminan el historial real.
8. Las muestras de tráfico se compactan según el perfil de uso, manteniendo disponible
   el modo de alta fidelidad.
9. Cada reducción se demuestra con una comparación reproducible antes/después.

## Reglas no negociables

- **No perder funciones para alcanzar una cifra.** Si una meta de tamaño requiere
  eliminar una función vigente, se conserva la función y se documenta el límite.
- **Separar UI de captura.** Ocultar o pausar render e IPC nunca detiene la observación
  nativa solicitada por la persona usuaria.
- **Sin autoinicio de captura.** La aplicación abre con captura desactivada salvo que
  exista una preferencia explícita de autostart ya aceptada.
- **Errores reales sí se muestran.** Quitar el mensaje de inicio no significa ocultar
  fallos de almacenamiento, permisos, captura o migración.
- **El código se elimina con evidencia.** Antes de borrar un módulo, comando, selector,
  traducción o dependencia se comprueba su alcance en TypeScript, Rust, tests, scripts,
  targets Windows/Linux y documentación.
- **Nada de red remota por defecto.** Se mantienen el procesamiento local, el broker de
  red y `offline_enforced`.
- **No degradar seguridad ni datos.** No se elimina validación, límites, migraciones,
  backups, procedencia de evidencia, integridad `.sabi` ni aislamiento del helper para
  ahorrar bytes.
- **Medir en el mismo entorno.** Las comparaciones registran commit, build de release,
  SO, CPU, RAM, WebView, permisos, dataset, duración y configuración.
- **Optimizar el producto, no solo el repositorio.** Separar archivos grandes mejora la
  mantenibilidad, pero solo cuenta como aligeramiento si reduce trabajo, memoria, I/O o
  artefactos medidos.

## Línea base observada

Estas cifras proceden del árbol y los artefactos 0.4.1 disponibles el 16 de agosto de
2026. `BASE-01` debe regenerarlas desde un build limpio antes de implementar cambios.

| Elemento | Línea base | Hallazgo |
| --- | ---: | --- |
| Workspace local sin `.git` | **~28,1 GiB** | Casi todo es caché/artefacto regenerable, no código ni datos de usuario |
| `target` de Cargo | **29.866.606.861 B / 27,815 GiB** | Causa directa de los “casi 30 GB” observados |
| `target/debug` | **26.851.258.608 B / 25,01 GiB** | 12,11 GiB incremental + 11,39 GiB deps + 1,44 GiB build |
| `target/release` | **2.706.190.232 B / 2,52 GiB** | Intermedios de compilación; el `.exe` final solo ocupa 20,6 MB |
| Target Linux cruzado | **307.099.776 B / 0,29 GiB** | Artefactos regenerables del cross-check |
| `node_modules` | **141.460.325 B / 0,132 GiB** | No explica el problema principal |
| Releases locales duplicadas | **167.931.704 B / 0,156 GiB** | Se pueden archivar o limpiar conservando la release necesaria |
| `apps/desktop/dist` | **9.612.676 B** | Todo se incorpora al frontend de Tauri |
| Source maps de producción | **5.228.126 B** | 54,4 % de `dist`; no son necesarios en el paquete público |
| `sabicheiro-logo.png` | **2.405.703 B** | Imagen 1254×1254 mostrada a 32×32; ~6 MiB al decodificarla en RGBA |
| JavaScript total generado | **1.901.508 B** | Incluye entrada y chunks diferidos |
| Entrada JS | **456.606 B** | Incluye vistas y backend de previsualización que no hacen falta al arrancar Tauri |
| CSS generado | **76.766 B** | Hoja única con estilos activos y estilos de pantallas sin ruta |
| Chunk Map 2D | **762.855 B** | GeoJSON/TopoJSON de 50 m; ya es diferido, pero debe compactarse |
| Chunk Globe 3D | **678.128 B** | Three.js + geometría de 110 m; ya es diferido |
| Ejecutable portable Windows | **20.589.568 B** | Incluye el frontend actual |
| Instalador NSIS Windows | **8.689.212 B** | Referencia para comparar el siguiente release |
| Historia de 100.000 filas | **136.632.519 B JSON** | Serialización p95 554,9883 ms; demasiado dato para un único cruce IPC |

Puntos de trabajo confirmados en el código actual:

- `useSabicheiro.ts` solicita un `DashboardUpdate` cada **400 ms** aunque no haya
  cambios y el `setInterval` puede iniciar otra petición antes de terminar la anterior.
- El motor Rust despierta cada **200 ms**, incluso con captura desactivada.
- Cada petición de dashboard genera snapshot, colecciones derivadas, diff y clones de
  estado aunque la revisión útil no haya cambiado.
- Con captura activa, `persist()` recoge y guarda todas las conexiones cada **1 s**,
  además de muestras y baselines.
- `processRateHistory` se recalcula y provoca estado React nuevo en cada actualización,
  pero ninguna pantalla conectada lo consume.
- `ProcessView.tsx` y `DestinationView.tsx` no tienen importador ni ruta activa.
- La demo contiene **8 destinos** y genera **1–3 eventos por tick**. Como
  `ephemeral_capture` es `false` por defecto, una sesión Demo puede terminar guardando
  conexiones, evidencia y muestras en la misma SQLite que el historial real.
- `traffic_samples` recibe cada segundo una muestra global y una por cada proceso
  activo con contadores por flujo; la retención predeterminada es de siete días.
- Las 1.525 líneas de datos y comportamiento de previsualización de `backend.ts` se
  deciden con una rama en runtime, por lo que forman parte del build normal.
- `list_handshake_mode_audit`, `list_network_actions` y
  `replay_evidence_projection` están registrados como comandos Tauri, pero no tienen
  llamada desde el frontend actual. Son candidatos a auditoría, no borrados automáticos.

El benchmark existente de 10–11 MiB mide el modelo Rust aislado, no la aplicación
Tauri/WebView completa. No se usará como línea base de RAM de escritorio.

Los casi 30 GB no corresponden a la aplicación instalada: proceden de `target`, que
contiene objetos, PDB, dependencias e incrementales regenerables. Esto no resta
importancia al problema. Esta hoja trata por separado el **peso distribuido** que recibe
la persona usuaria y el **peso de trabajo** que ocupa el repositorio al desarrollar.

## Estados y prioridades

| Estado | Significado |
| --- | --- |
| `TODO` | No iniciado |
| `IN_PROGRESS` | En implementación; aún no cumple toda la aceptación |
| `REVIEW` | Implementado; falta comparación o validación nativa |
| `BLOCKED` | Existe un bloqueo externo reproducible y una siguiente acción escrita |
| `DONE` | Implementación, limpieza, pruebas, medición y documentación completas |

| Prioridad | Significado |
| --- | --- |
| `P0` | Obligatorio para publicar la versión ligera |
| `P1` | Mejora importante que se ejecuta tras los ahorros seguros |
| `P2` | Solo se realiza si la medición demuestra beneficio neto |

## Metas de salida

| Métrica | Objetivo mínimo |
| --- | --- |
| Mensaje de inicio | **0** montajes de `FirstRun`; entrada directa en Today |
| Funciones visibles | **100 %** de la matriz de paridad pasa |
| Frontend empaquetado | **≤3 MiB** y **0 `.map`** dentro de `frontendDist` de release |
| Recursos críticos iniciales | **≤750 KiB** antes de abrir mapa/globo |
| Portable Windows | **≤16 MiB** o reducción ≥20 % con causa del límite documentada |
| Instalador Windows | **≤7,5 MiB** o reducción ≥15 % con causa documentada |
| Espacio tras limpieza completa | `target` **0 B** y workspace regenerable **<500 MiB** sin borrar fuentes/datos |
| Espacio tras un gate completo limpio | `target` **≤8 GiB**; aviso obligatorio a 6 GiB y bloqueo de acumulación a 10 GiB |
| Recuperación inicial de disco | Liberar de forma segura **≥24 GiB** de artefactos regenerables |
| Arranque hasta UI utilizable | Reducción p50 y p95 **≥20 %** en frío y caliente |
| CPU con captura off | Reducción **≥50 %**; sin bucle fijo de UI a 400 ms ni motor ocioso a 200 ms |
| CPU/RAM con captura live | Sin regresión >5 % y sin crecimiento sostenido en soak de 8 h |
| IPC de dashboard | **0 solapamientos**; no serializar snapshot completo si no hay cambio |
| Escrituras en escenario estable | Reducción de bytes escritos **≥50 %** sin perder durabilidad acordada |
| Datos de ejemplo en SQLite real | **0 filas nuevas** al usar Demo; base nueva sin contenido precargado |
| Tamaño de la demo | **≤4 destinos** y cadencia acotada, conservando TCP/UDP, procesos y geografía |
| Muestras de tráfico | Reducción de filas/bytes **≥70 %** en perfil normal estable; detalle de 1 s disponible en alta fidelidad |
| Historia 100k | Primera página **<1 MiB**, interacción p95 **≤100 ms**, exportación completa local |
| Captura y evidencia | Mismos IDs, cierres, contadores, gaps y procedencia para las mismas fixtures |
| Privacidad | **0 egress** al abrir y en el escenario por defecto |

Las metas relativas se comparan en el mismo equipo. Si una cifra absoluta no es viable
sin perder funciones, el release exige conservar la función, adjuntar el perfil y
explicar el límite; no se maquilla la medición.

## Resumen de ejecución

| ID | Trabajo | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| BASE-01 | Medición reproducible y matriz de paridad | P0 | BLOCKED | Permiso de ejecución GUI agotado; sonda versionada |
| SPACE-01 | Recuperar y limitar el espacio de compilación | P0 | DONE | BASE-01 |
| START-01 | Quitar mensaje/panel de inicio sin ocultar controles | P0 | DONE | BASE-01 |
| PACK-01 | Excluir source maps del producto | P0 | DONE | BASE-01 |
| PACK-02 | Sustituir el logo sobredimensionado | P0 | REVIEW | Escalas nativas 125–200 % pendientes |
| DEMO-01 | Reducir y aislar ejemplos/datos de demostración | P0 | DONE | BASE-01 |
| CLEAN-01 | Eliminar código muerto del frontend | P0 | DONE | BASE-01, START-01 |
| PACK-03 | Separar backend Tauri y previsualización de desarrollo | P0 | DONE | CLEAN-01 |
| LOAD-01 | Carga diferida por vista y bootstrap mínimo | P0 | DONE | PACK-03 |
| IPC-01 | Actualizaciones secuenciales, adaptativas y dirigidas por cambios | P0 | DONE | BASE-01 |
| CORE-01 | Snapshot/delta incremental sin clones completos innecesarios | P0 | DONE | IPC-01 |
| RENDER-01 | Suspender render y WebGL cuando no son visibles | P0 | REVIEW | Soak nativo de 50 cambios pendiente |
| DATA-01 | Persistencia incremental y por lotes | P0 | DONE | BASE-01 |
| CLEAN-02 | Auditar comandos, crates y dependencias sin consumidores | P1 | DONE | CLEAN-01 |
| PACK-04 | Compactar geometría y chunks de mapa/globo | P1 | DONE | LOAD-01 |
| DATA-02 | Paginar historia y reducir IPC masivo | P1 | DONE | CORE-01, DATA-01 |
| DATA-03 | Compactar muestras de tráfico y crecimiento de SQLite | P1 | DONE | BASE-01, DATA-01 |
| CORE-02 | Motor ocioso por eventos y próximos deadlines | P1 | DONE | CORE-01, DATA-01 |
| BUILD-01 | Afinar perfil release y dependencias Rust | P1 | DONE | CLEAN-02 |
| CLEAN-03 | Depurar CSS, i18n y assets restantes | P2 | DONE | CLEAN-01, LOAD-01 |
| QA-01 | Gates completos, artefactos y comparación final | P0 | BLOCKED | Launch/soak/instalación nativa no autorizados |

### Evidencia de ejecución 0.4.2 (2026-08-16)

- `docs/performance-0.4.2-before.json` y `docs/performance-0.4.2-after.json`
  registran entorno, commit base, hashes de fixtures, assets, dependencias, espacio y
  artefactos. El frontend baja de 9.612.676 B a 1.163.691 B, los recursos iniciales de
  2.939.648 B a 398.749 B y los source maps de 5.228.126 B a cero.
- El registro histórico midió 6.528.512/2.851.022 B para Portable/NSIS y comparó
  `z`/`s` como 6.528.000/7.347.200 B. La auditoría de 2026-08-22 demostró que aquel
  portable usaba `cfg(dev)` y omitía `frontendDist`; se conserva como evidencia del
  experimento, no como baseline distribuible. La referencia Tauri corregida y los
  tamaños actuales están en el addendum anterior y en `docs/VALIDATION.md`.
- La fixture SQLite de 100.000 filas devuelve 500 filas/716.705 B en 5,77 ms p95,
  filtra en 4,26 ms p95, recorre 201 páginas/100.000 IDs únicos en 800,84 ms y exporta
  las 100.000 filas en 1.365,52 ms.
- La simulación normal de ocho horas baja de 2.880 filas crudas a ≤24 filas
  crudas/rollup (≥99,16 %), con bytes y tasas exactos; el perfil de 1 s conserva todas
  sus filas. Demo simula diez minutos con ≤4 conexiones, ≤1 evento/s y cero filas reales.
- Pasan TypeScript estricto, 926 claves españolas sin huérfanas, auditoría CSS/imports,
  paridad de 60 comandos IPC, 46 dependencias Rust directas, licencias bloqueadas,
  formato, Clippy estricto, source-check Linux, 133 tests y NSIS/SBOM/hashes.
- La revisión visual de preview cubrió Today directo, captura off, Demo, tabla, mapa,
  globo, History, Exposure, Data & Health e inglés/castellano sin errores de consola.
  No sustituye evidencia nativa.
- `BASE-01`, `PACK-02`, `RENDER-01` y `QA-01` no se marcan `DONE`: el servicio de
  aprobación rechazó abrir el ejecutable local al agotarse su cuota. Quedan sin afirmar
  diez arranques frío/caliente, los cinco perfiles de cinco minutos, escalas HiDPI
  nativas, 50 cambios Map↔Globe, ciclo instalar/desinstalar, ETW elevado, paquetes Linux
  y soak de ocho horas. La siguiente acción exacta, cuando vuelva a autorizarse GUI, es
  `node scripts/measure-windows-launch.mjs --iterations 10`, seguida de la matriz nativa
  descrita en `docs/VALIDATION.md`; no se han inventado resultados para cerrar estados.

## Detalle de tareas

### BASE-01 — Medir antes de tocar

- Añadir un script reproducible que haga build limpio y registre tamaño de cada asset,
  `dist`, binario, instalador, dependencias y chunks.
- Medir al menos diez arranques fríos y diez calientes hasta primer frame y hasta UI
  interactiva.
- Perfilar durante cinco minutos cada escenario: captura off visible, captura off en
  bandeja, live visible en 2D, live visible en 3D y live en bandeja.
- Registrar CPU, working set/RSS, GPU cuando esté disponible, handles/threads, IPC por
  segundo, bytes serializados, lecturas/escrituras y tamaño/WAL de SQLite.
- Ejecutar una fixture fija y guardar hashes de conexiones/evidencia antes de optimizar.
- Crear una matriz de paridad con los recorridos: iniciar/parar live y demo, cambiar
  interfaz, pausar vista, buscar, mapa, globo, Today, historial/time machine,
  investigación `.sabi`, Exposure, reglas, notificaciones, grabación, GeoIP,
  retención, exportación, bandeja, autostart, idioma y accesibilidad por teclado.

**Aceptación:** existe un informe versionado antes/después, generado con un único
comando, y ninguna tarea de rendimiento se declara `DONE` sin sus cifras.

### SPACE-01 — El proyecto no puede volver a ocupar 30 GB

- Añadir un informe de espacio que separe fuentes, datos de usuario, `target/debug`,
  incrementales, dependencias, `target/release`, bundles, targets cruzados,
  `node_modules`, store pnpm y releases locales.
- Añadir comandos de limpieza con `dry-run` y alcance visible: limpieza incremental,
  limpieza de debug obsoleto, limpieza de release y limpieza completa. Antes de borrar,
  deben resolver y comprobar que el destino exacto está dentro de este workspace.
- La limpieza completa puede usar `cargo clean` porque `target` es regenerable, pero
  requiere una acción explícita; nunca borra código, lockfiles, documentación, bases de
  datos de usuario, GeoIP importado ni artefactos que la persona haya marcado para
  conservar.
- Recuperar inicialmente al menos 24 GiB eliminando objetos obsoletos de `target`.
  Conservar solo los artefactos finales que se necesiten para comparar la release.
- Comparar `[profile.dev]` y `[profile.test]` con información de depuración limitada a
  líneas, y desactivar incremental en CI/gates de una sola ejecución. No empeorar sin
  medir la experiencia normal de compilación local.
- Evitar que el chequeo cotidiano construya innecesariamente todos los ejemplos,
  binarios auxiliares y targets. Mantener el gate completo en CI/release y ofrecer un
  gate rápido local; no eliminar ninguna prueba.
- No mezclar builds incompatibles en el mismo directorio indefinidamente. Separar o
  limpiar perfiles/targets cuando cambien toolchain, features o plataforma.
- Mostrar aviso al superar 6 GiB y un informe accionable al superar 10 GiB. La
  herramienta no borra automáticamente: propone exactamente qué caché regenerable se
  puede limpiar y cuánto liberará.
- Mantener como máximo la release local actual y la línea base necesaria para comparar;
  archivar o borrar duplicados antiguos solo con confirmación, conservando hashes y
  documentos versionados.

**Aceptación:** una limpieza explícita deja `target` vacío y el workspace regenerable
por debajo de 500 MiB; tras un build + test + bundle limpios, `target` no supera 8 GiB,
el producto sigue compilando desde cero y se han recuperado al menos 24 GiB.

### START-01 — Quitar el mensaje que aparece al iniciar

- Retirar los dos montajes de `FirstRun` en `App.tsx`: Today y el lienzo de Live.
- Abrir directamente en Today con la captura en su estado real, sin modal, tarjeta de
  bienvenida ni inicio automático de demo/live.
- Mantener el selector Capture off / Live capture / Demo data y el selector de interfaz
  en `TopBar`; el idioma continúa disponible en Data & Health.
- Mantener estados vacíos discretos dentro de cada vista y banners únicamente para
  errores reales y accionables.
- Borrar `FirstRun.tsx`, sus selectores `.first-run*` y `.today-onboarding`, y solo las
  traducciones que queden sin consumidor.
- Conservar y probar `windows_subsystem = "windows"` para que el release de Windows no
  abra una consola adicional.

**Aceptación:** primer inicio, inicio posterior, base vacía y captura detenida abren la
interfaz normal sin el mensaje; demo/live siguen iniciándose desde TopBar y los fallos
reales siguen visibles.

### PACK-01 — No empaquetar source maps

- Hacer `sourcemap` dependiente de un modo de diagnóstico explícito y dejarlo
  desactivado en el build de Tauri/release.
- Si se necesitan para depurar, publicarlos como artefacto privado de CI con el mismo
  hash de commit, nunca dentro de `frontendDist`, portable o instalador.
- Añadir un gate que falle si el bundle público contiene `.map`, fuentes del proyecto o
  rutas locales.

**Aceptación:** se eliminan al menos 5.228.126 B de `dist`, los stack traces minificados
siguen siendo simbolizables con el artefacto privado y el build normal no cambia.

### PACK-02 — Logo adecuado a su uso

- Generar desde la fuente actual un recurso nítido de 64×64 o 128×128, optimizado y con
  el mismo aspecto; reutilizar un icono Tauri existente si supera la comparación visual.
- Evitar decodificar una imagen 1254×1254 para mostrarla a 32×32.
- Comprobar 100/125/150/200 % de escala, tema y fondo antes de borrar el PNG antiguo.

**Aceptación:** el logo visible no supera 25 KiB, no hay cambio visual apreciable y
`sabicheiro-logo.png` de 2.405.703 B deja de formar parte del producto.

### DEMO-01 — Ejemplos mínimos y fuera de la base real

- Garantizar mediante test que una base nueva contiene cero sesiones, conexiones,
  evidencia, muestras, investigaciones, reglas y notificaciones de ejemplo.
- Reducir `DemoGenerator` de ocho destinos a un máximo de cuatro casos genéricos que
  sigan cubriendo al menos dos procesos, TCP, UDP, varios países/ASN, apertura,
  actualización y cierre.
- Reducir la generación de 1–3 eventos cada tick de 200 ms a una cadencia visible pero
  acotada: como máximo un evento por segundo y cuatro conexiones activas.
- No usar marcas, dominios ni IP públicas reales si un nombre `.invalid` y rangos de
  documentación permiten demostrar lo mismo sin aparentar tráfico real.
- Marcar el origen de sesión de forma explícita y ejecutar Demo en almacenamiento
  temporal/en memoria. Si Investigate necesita consultar la demo, debe leer ese almacén
  aislado mientras la demo está activa; nunca mezclarla con el historial capturado.
- Mantener como máximo una sesión de demostración temporal y sustituirla al reiniciar
  Demo. Salir de Demo o de la aplicación libera sus filas y buffers.
- Detectar sesiones Demo antiguas únicamente cuando toda su evidencia/origen lo pruebe.
  Data & Health debe mostrar cuántas filas y bytes ocupan y ofrecer borrarlas con
  confirmación; no eliminar automáticamente una sesión ambigua o datos reales.
- Reutilizar el mismo conjunto mínimo en el preview de desarrollo y excluirlo por
  completo del release mediante `PACK-03`.
- Conservar fixtures `#[cfg(test)]`, el harness y el ejemplo de benchmark que no se
  empaquetan en el ejecutable. Se pueden consolidar para reducir mantenimiento, pero no
  se contabilizan falsamente como ahorro del producto.

**Aceptación:** diez minutos de Demo no hacen crecer la SQLite real, la demo continúa
permitiendo probar mapa, globo, tabla, filtros e Investigate, y la limpieza de demos
anteriores nunca selecciona una sesión real.

### CLEAN-01 — Código muerto del frontend

- Eliminar `ProcessView.tsx` y `DestinationView.tsx`, actualmente sin importador ni
  ruta, después de confirmar que sus capacidades vigentes existen en Live,
  Investigate o el inspector.
- Eliminar `processRateHistory` de `useSabicheiro.ts`: hoy se calcula, copia y devuelve
  en cada actualización sin consumidor activo.
- Auditar métodos de `desktopApi`, exports, props, tipos, iconos, traducciones y
  selectores asociados; no borrar clases compartidas como `process-avatar`.
- Activar `noUnusedLocals` y `noUnusedParameters` y añadir un chequeo del grafo de
  imports desde `main.tsx`, incluyendo imports dinámicos.
- Cada supresión debe indicar consumidor anterior, sustituto funcional si lo hay y
  diferencia de bundle/runtime; el código que Vite ya excluía no se presenta como ahorro
  de paquete.

**Aceptación:** el frontend no contiene módulos inalcanzables ni trabajo React sin
consumidor, TypeScript estricto pasa y la matriz funcional permanece verde.

### PACK-03 — Backend de producción separado del preview

- Dividir `backend.ts` en contrato tipado, adaptador Tauri y adaptador de preview.
- Seleccionar el adaptador con configuración de build, no con una rama runtime que
  obligue a empaquetar ambos.
- Mantener la previsualización determinista para desarrollo mediante un comando o modo
  explícito; el build Tauri de release no contiene destinos, snapshots ni respuestas de
  preview.
- Evitar duplicar las 66 operaciones: compartir tipos y generar o comprobar la paridad
  del contrato.

**Aceptación:** preview sigue funcionando en desarrollo, release usa exclusivamente
IPC Tauri y un análisis del chunk demuestra que los datos/strings de preview no están
en el producto.

### LOAD-01 — Arranque y carga por vista

- Mantener Today y el shell como ruta crítica; cargar Investigate, Exposure, Data &
  Health, inspector y diálogos pesados solo al abrirlos.
- Conservar Map y Globe como chunks independientes y no precargar Three.js si se abre
  Today o una vista sin visualización.
- Sustituir las cinco llamadas iniciales independientes por un bootstrap coherente o
  diferir GeoIP, grabación y reglas hasta que la primera pintura esté lista, sin mostrar
  valores falsos.
- Evitar que una función lenta bloquee toda la aplicación: shell primero, estados
  parciales accesibles y error localizado después.

**Aceptación:** la UI base es utilizable antes de cargar vistas secundarias, el asset
crítico cumple el límite y abrir cada chunk por primera vez mantiene foco y estado.

### IPC-01 — De polling fijo a actualizaciones útiles

- Como corrección inmediata, reemplazar el `setInterval` async por un ciclo secuencial
  con una única petición en vuelo y backoff acotado.
- Preferir eventos Tauri o long-poll para notificar revisiones de dashboard, cambios de
  captura y diagnósticos; conservar resync por secuencia ante pérdida.
- Cuando la captura esté off o la ventana oculta, detener actualizaciones visuales
  periódicas y usar solo eventos/heartbeat de salud de baja frecuencia.
- Al volver a primer plano, pedir una única resincronización y continuar desde ella.
- Medir peticiones, bytes, deltas vacíos, solapamientos, latencia y resyncs en Data &
  Health.

**Aceptación:** no existen llamadas solapadas ni polling UI fijo oculto/off; un cambio
visible llega en ≤500 ms, y tormenta, pérdida y reanudación no omiten conexiones.

### CORE-01 — Snapshot y delta incrementales

- Añadir dirty flags/revisiones separadas para conexiones, agregados, captura,
  diagnósticos y sesión.
- Si no cambió nada solicitado, devolver un delta vacío pequeño sin reconstruir ni
  clonar el snapshot completo.
- Mantener conjuntos de IDs modificados/eliminados o una cola acotada de deltas en vez
  de crear `HashMap`, `HashSet` y copia completa para cada cliente cada 400 ms.
- Recalcular procesos, destinos y estadísticas solo cuando cambien sus entradas.
- Conservar secuencia, orden determinista, detección de gap y fallback a snapshot
  completo.

**Aceptación:** fixture idéntica produce el mismo estado y hashes; en estado estable no
hay clones/serialización proporcional al número total de conexiones.

### RENDER-01 — No renderizar lo que no se ve

- Pausar `requestAnimationFrame` del globo cuando el documento/ventana no sea visible y
  reanudar con un frame coherente.
- Liberar renderer, geometrías, materiales, observers y listeners al salir de Globe o
  cerrar la ventana; probar cambios repetidos Globe ↔ Map.
- Evitar `setState` y rerender global para deltas vacíos; estabilizar callbacks y
  separar stores/selecciones por vista si el perfil de React lo justifica.
- Limitar rutas y device pixel ratio solo con comparación visual y preferencia de
  calidad; `reduce_motion` debe reducir trabajo real, no solo animación CSS.
- Mantener tabla/alternativa accesible cuando WebGL no exista.

**Aceptación:** al ocultar la ventana no quedan frames WebGL continuos, 50 cambios de
vista no aumentan memoria/GPU y mapa, globo, selección y accesibilidad conservan su
comportamiento.

### DATA-01 — Escribir únicamente cambios

- Marcar conexiones nuevas, modificadas y cerradas, y hacer upsert solo de ese lote.
- Separar la cadencia de lifecycle, muestras, resumen de sesión y baseline; no volver a
  serializar/guardar baselines idénticos cada segundo.
- Reutilizar statements/transacciones y mover trabajo pesado fuera del lock principal
  sin reordenar evidencia.
- Forzar flush al pausar/detener, salir, completar grabación y antes de operaciones de
  mantenimiento.
- Definir y probar la ventana máxima de pérdida tras crash; conservar WAL, migraciones,
  backup, salvage y modo ephemeral.

**Aceptación:** datos e investigaciones se reconstruyen igual tras cierre normal y
fault injection; el escenario estable reduce ≥50 % los bytes escritos y no aumenta la
latencia de captura.

### CLEAN-02 — Comandos y dependencias sin uso

- Comparar automáticamente comandos de `generate_handler!`, métodos del contrato TS y
  consumidores reales. Revisar primero `list_handshake_mode_audit`,
  `list_network_actions` y `replay_evidence_projection`.
- Si un comando solo sirve a tests o mantenimiento, moverlo tras feature/dev gate; si
  representa una función visible, conectarlo únicamente si ya pertenece al producto.
- Ejecutar `cargo tree`, Clippy y una herramienta de dependencias no usadas, tratando
  resultados `cfg(windows)`/`cfg(unix)` como candidatos que requieren revisión manual.
- Revisar features de Tauri, sysinfo, reqwest, rusqlite y crates de plataforma. No
  eliminar TLS, SQLite bundled, tray o proveedores nativos sin alternativa equivalente.
- Eliminar del manifest y lockfile solo dependencias sin símbolos/rutas necesarias.

**Aceptación:** no hay comandos registrados sin propósito documentado ni dependencia
directa sin consumidor; Windows y Linux siguen compilando sus rutas específicas.

### PACK-04 — Geometría y visualizaciones

- Medir por separado Three.js, OrbitControls, TopoJSON y los datasets de 50 m/110 m.
- Precalcular en build las rutas/buffers de fronteras y evitar enviar JSON general y
  `topojson-client` al runtime si el resultado es menor.
- Compartir una geometría compacta entre 2D/3D cuando conserve el detalle visible, o
  mantener niveles distintos si la comparación demuestra que son necesarios.
- Usar imports de Three.js que permitan tree-shaking y probar una versión reducida solo
  si conserva cámara, selección, rutas, movimiento reducido y liberación de GPU.
- No sustituir mapa o globo por una captura estática ni cargar assets remotos.

**Aceptación:** ambos modos pasan snapshots visuales e interacción y sus chunks bajan
al menos 20 %, o la tarea documenta con perfil por qué una reducción segura no existe.

### DATA-02 — Historia paginada, no un JSON de 136 MB

- Añadir cursor estable y paginación en SQLite/backend, con filtro y orden tipados del
  lado nativo.
- Cargar una ventana pequeña en la tabla virtualizada y precargar páginas adyacentes
  con cancelación; una búsqueda nueva invalida respuestas obsoletas.
- Mantener recuentos y agregados sin materializar 100.000 objetos en React.
- Ejecutar exportación completa y reconstrucciones grandes en Rust/streaming, no
  pasando todo el dataset por el webview.
- Preservar selección, time machine, comparaciones e investigaciones guardadas mediante
  IDs estables.

**Aceptación:** se pueden recorrer y exportar las mismas 100.000 filas, la primera
respuesta es <1 MiB, filtro/scroll p95 ≤100 ms y memoria del webview queda acotada.

### DATA-03 — Menos muestras sin perder el modo preciso

- Medir filas y bytes por minuto de `traffic_samples`, separados en globales y por
  proceso, con 0, 10, 100 y 1.000 procesos observados.
- Hacer que la cadencia dependa del perfil: alta fidelidad conserva 1 s; el perfil
  normal usa una cadencia menor y background agrupa más, con el valor real visible en
  la UI.
- No insertar filas idénticas sin límite. Guardar cambios más un heartbeat acotado o
  usar tramos con duración, de forma que las consultas puedan reconstruir exactamente
  el intervalo representado.
- Mantener detalle crudo corto y rollups de minuto/hora para periodos largos. Quien
  necesite detalle de 1 s durante toda la retención puede elegir alta fidelidad; no se
  elimina esa capacidad.
- No conservar muestras por proceso cuando el proveedor solo ofrece contador agregado
  de interfaz, ni crear filas cero para procesos inactivos.
- Mostrar antes de aplicar retención la estimación por tabla y liberar espacio con
  checkpoint/compactación segura después del borrado, nunca durante una captura intensa.
- Migrar el esquema con backup/rollback y comprobar sumas de bytes, paquetes, máximos y
  medias entre datos crudos y compactados.

**Aceptación:** el perfil normal reduce al menos 70 % las filas/bytes en un escenario
estable de ocho horas, las gráficas y totales permanecen dentro de la precisión definida
y alta fidelidad conserva consultas reales de un segundo.

### CORE-02 — Motor ocioso dirigido por eventos

- Sustituir el wakeup fijo de 200 ms por espera hasta el siguiente deadline o una señal
  de comando, proveedor, DNS, GeoIP, grabación o shutdown.
- Mantener la cadencia requerida durante live; con captura off no despertar cinco veces
  por segundo para no hacer trabajo.
- Calcular el deadline mínimo de captura, persistencia, diagnóstico, interfaz,
  retención y backoff del supervisor.
- Despertar inmediatamente ante una orden Tauri o resultado de worker y verificar
  sleep/resume.

**Aceptación:** desaparece el tick fijo en reposo, los comandos conservan respuesta
inmediata y las métricas de recall/cierre de las fixtures no empeoran.

### BUILD-01 — Perfil release y peso nativo

- Obtener desglose de símbolos/secciones y dependencias del ejecutable antes de cambiar
  flags.
- Comparar `opt-level=s/z`, LTO thin/fat y `panic=abort` en tamaño, arranque, CPU y
  fixtures; elegir el punto Pareto, no el binario más pequeño a cualquier coste.
- Revisar features transitivas y las varias generaciones de bindings Windows; actualizar
  o unificar solo con compatibilidad demostrada.
- Perfilar el stack HTTP de la actualización GeoIP. Sustituir/configurar reqwest solo si
  el cliente alternativo conserva HTTPS, timeouts, límites, redirects controlados,
  checksum, proxy y tests offline.
- Mantener símbolos de depuración fuera del artefacto público y asociados al commit.

**Aceptación:** el portable/instalador alcanza la meta o presenta un desglose verificable
del límite; no hay regresión >5 % de CPU/latencia ni pérdida de seguridad.

### CLEAN-03 — CSS, i18n y assets residuales

- Generar lista de selectores y claves sin consumidor después de dividir las vistas.
- Eliminar únicamente reglas huérfanas; conservar selectores construidos dinámicamente
  mediante allowlist explícita.
- Dividir estilos/traducciones por vista solo si reduce el asset inicial más de lo que
  añade en requests, complejidad y flashes visuales.
- Buscar PNG/ICO, fixtures, docs y schemas duplicados en el producto final, distinguiendo
  recursos de build de recursos de runtime.

**Aceptación:** auditorías CSS/i18n pasan en inglés y castellano, no hay FOUC ni cambios
visuales y la reducción medida justifica la complejidad añadida.

### QA-01 — Cierre y release

- Ejecutar formato, TypeScript estricto, auditoría i18n, build Vite, Clippy estricto y
  todos los tests Rust con lockfiles.
- Ejecutar matriz de paridad y fixtures con hashes antes/después; cualquier diferencia
  de datos bloquea la release hasta explicarse.
- Ejecutar Windows normal y aprobado, Linux nativo disponible, accesibilidad por teclado,
  escalas HiDPI, WebGL/no-WebGL, offline y egress.
- Completar tormenta, 100.000 filas, 8 h, bandeja, sleep/resume y cambio de red.
- Construir portable/NSIS y AppImage/deb aplicables; comprobar launch, update,
  instalación, desinstalación y retención de datos.
- Regenerar SBOM, hashes, licencias, notas, `docs/VALIDATION.md` y cifras de esta hoja.

**Aceptación:** todas las P0 están `DONE`, las P1 tienen resultado medido aceptado, no
hay regresión funcional y los artefactos cumplen las metas o documentan honestamente el
límite sin eliminar funciones.

## Orden obligatorio

1. **Congelar evidencia y recuperar disco:** BASE-01 y SPACE-01.
2. **Ahorros seguros, inicio y ejemplos:** START-01, PACK-01, PACK-02 y DEMO-01.
3. **Eliminar trabajo sin consumidor:** CLEAN-01 y PACK-03.
4. **Reducir el arranque:** LOAD-01.
5. **Reducir trabajo continuo:** IPC-01, CORE-01 y RENDER-01.
6. **Reducir disco y datasets masivos:** DATA-01, DATA-02 y DATA-03.
7. **Afinar binario y visualizaciones:** CLEAN-02, PACK-04, CORE-02 y BUILD-01.
8. **Limpieza residual:** CLEAN-03, únicamente con beneficio probado.
9. **Cerrar:** QA-01 y artefactos finales.

No se mezclan todos los cambios en un único commit. Cada bloque conserva una medición
antes/después y se puede revertir sin arrastrar los demás.

## Protocolo para eliminar algo

1. Localizar importadores, llamadas IPC, registro Tauri, usos dinámicos, tests, scripts,
   CSS, traducciones y documentación.
2. Confirmar comportamiento sustituto o demostrar que no existe ruta de usuario.
3. Borrar código, tests exclusivos obsoletos, estilos, claves y dependencia asociada en
   el mismo cambio.
4. Compilar Windows y hacer source-check/cross-check de Linux; no interpretar un `cfg`
   inactivo como código muerto.
5. Ejecutar la matriz afectada y comparar bundle, CPU/RAM/I/O o complejidad.
6. Revertir la eliminación si solo reduce líneas pero introduce duplicación, fallo o
   pérdida funcional.

## Criterio de salida

La versión ligera no se publica mientras ocurra cualquiera de estos puntos:

- aparece `FirstRun` o cualquier mensaje promocional al iniciar;
- una función de 0.4.1 deja de ser alcanzable;
- el bundle público contiene source maps o el logo de 2,4 MB;
- `target` vuelve a crecer por encima de 10 GiB sin aviso y desglose accionable;
- no existe una limpieza segura capaz de recuperar los artefactos regenerables;
- siguen existiendo polling async solapado o trabajo proporcional a todo el estado ante
  un delta vacío;
- la captura se pausa porque la ventana se oculta;
- una optimización cambia conexiones, evidencia, cierres, contadores o procedencia;
- se borra código específico de Windows/Linux sin compilar y probar su target;
- el historial grande vuelve a cruzar IPC como un único JSON sin límite;
- Demo inserta ejemplos en la SQLite real o una base nueva trae filas precargadas;
- la compactación de muestras oculta su resolución o impide activar detalle de 1 s;
- una reducción depende de ocultar errores, debilitar seguridad o eliminar migraciones;
- no existen artefactos y métricas comparables antes/después.

## Fuera de alcance de esta iteración

- Añadir nuevos proveedores de captura, un rediseño visual completo o funciones cloud.
- Eliminar mapa, globo, demo, historial, Exposure, bandeja, GeoIP o grabación para bajar
  el tamaño.
- Sustituir datos locales por APIs, tiles, fuentes o telemetría remotos.
- Reducir retención o precisión silenciosamente.
- Minificar manualmente el código fuente o hacer refactors amplios sin beneficio medido.
- Presentar como ahorro del producto la limpieza de un archivo que ya era eliminado por
  tree-shaking.
