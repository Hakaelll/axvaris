# Sabicheiro: próximas mejoras significativas

Última actualización: **2026-08-14**
Versión objetivo: **0.4.0 — Process to Path**

Actualización de mantenimiento: **0.4.1 — Spain Focus** centra España al abrir/restaurar
el mapa y el globo, endurece Exposición frente a snapshots malformados o respuestas
asíncronas obsoletas y añade `docs.html`. No cambia el estado de los P0 nativos que
siguen abiertos en esta hoja de ruta.

Esta hoja de ruta parte de la versión 0.3.0 y no repite su trabajo ya terminado. La
identidad canónica de IP, el GeoIP local, la procedencia de evidencia, las
investigaciones guardadas, el IPC incremental y el mapa/globo actuales son la base. Su
historial se conserva en `CHANGELOG.md` y `docs/VALIDATION.md`.

La 0.4.0 debe ser un salto funcional: Sabicheiro no solo mostrará conexiones, sino que
explicará con evidencia **qué proceso las originó, qué resolución DNS pudo precederlas,
por qué interfaz o VPN salieron, cuánto duraron, si son nuevas para ese contexto y qué
cambió respecto a la actividad habitual**.

## Resultado de producto esperado

Al terminar esta hoja de ruta, una persona debe poder responder sin conocimientos de
redes avanzados:

1. **¿Qué aplicación se conectó y qué instancia concreta del proceso fue?**
2. **¿Qué conexiones muy breves se perderían con un snapshot y cuáles capturó el
   proveedor por eventos?**
3. **¿Qué nombre DNS, ruta, interfaz, VPN y pista de protocolo están realmente
   relacionadas con la conexión?**
4. **¿Es la primera vez que este proceso, ejecutable o perfil de red ve ese destino?**
5. **¿Qué servicios locales están escuchando y en qué redes podrían estar expuestos?**
6. **¿Qué cambió hoy y qué merece revisión, sin convertirlo en un falso veredicto de
   malware?**
7. **¿Cómo guardar, comparar y compartir una investigación local sin entregar datos a
   un servicio externo?**

## Principios no negociables

- **Sin APIs externas:** ninguna función depende de una API web, ni gratuita ni de
  pago. No hay claves, cuentas, cuotas ni envío de IP, dominio, hash o investigación a
  terceros. Los catálogos son embebidos o importados por la persona usuaria.
- **Local y sin coste de servicio:** no añadir telemetría, sincronización cloud, mapas
  alojados, suscripciones, pruebas temporales ni dependencias que exijan pago para
  compilar, probar o utilizar una función principal.
- **Evidencia antes que inferencia:** cada relación muestra fuente, instante, calidad y
  limitación. Una coincidencia por tiempo o puerto nunca se presenta como certeza.
- **Utilidad sin privilegios:** el modo normal debe seguir funcionando con IP Helper o
  `INET_DIAG`. La captura avanzada es opcional y su ausencia degrada capacidades
  concretas, no toda la aplicación.
- **Privilegio mínimo:** ningún webview ejecuta comandos, ningún helper acepta shell o
  rutas arbitrarias y la UI completa nunca se ejecuta como administrador/root.
- **Sin payload por defecto:** la observación normal conserva metadatos. Cualquier modo
  de metadatos de handshake es explícito, visible, acotado, no persistente y no
  descifra tráfico de aplicación.
- **Sin acciones peligrosas automáticas:** las reglas pueden etiquetar, fijar o avisar;
  nunca bloquean red ni comienzan una grabación de paquetes sin una acción separada.
- **Windows y Linux de escritorio:** Windows 10/11 x86_64 y Ubuntu/Debian x86_64 son
  los targets verificados. No reintroducir soporte móvil o macOS.
- **No inventar seguridad:** Sabicheiro describe comportamiento y cambios observados;
  no afirma que un proceso sea benigno o malicioso sin una fuente que no existe.

## Estados y prioridades

| Estado | Significado |
| --- | --- |
| `TODO` | No iniciado |
| `IN_PROGRESS` | Implementación activa; no cumple todavía toda la aceptación |
| `BLOCKED` | Bloqueo externo reproducible con siguiente acción documentada |
| `REVIEW` | Implementado, pero falta evidencia nativa, visual o de privilegios |
| `DONE` | Implementación, pruebas, documentación y evidencia completas |

| Prioridad | Significado |
| --- | --- |
| `P0` | Bloquea la release 0.4.0 |
| `P1` | Mejora principal; solo se aplaza con justificación explícita |
| `P2` | Mejora útil que no debe retrasar un núcleo ya validado |

Una tarea no pasa a `DONE` solo porque compile. Si cambia la interfaz debe incluir
inglés y castellano; si toca una fuente nativa debe tener fixture y prueba en el sistema
operativo correspondiente; si añade persistencia debe probar migración, rollback y
retención; si añade red debe documentar exactamente cuándo y por qué se usa.

## Métricas de salida acordadas

| Métrica | Objetivo mínimo de 0.4.0 |
| --- | --- |
| Falsos destinos remotos en fixtures | **0** listeners, wildcard o UDP sin peer convertidos en destino |
| Precisión proceso ↔ flujo controlado | **100 %**; una atribución dudosa queda desconocida |
| Recall del proveedor por eventos | **≥99 %** en flujos de ≥50 ms y **≥95 %** entre 5–50 ms en la fixture local |
| Detección de cierre por eventos | p95 **≤1 s**; el fallback declara su intervalo real |
| Pérdida del proveedor | contador y degradación visibles en **≤2 s** |
| Causalidad DNS confirmada | **≥99 %** de precisión; relaciones ambiguas permanecen como candidatas |
| Latencia de interacción | p95 **≤100 ms** para filtro/selección con 100.000 filas históricas |
| Captura en reposo | objetivo **<3 % CPU** y **<250 MiB RAM** en el equipo de referencia |
| Soak | **8 h**, colas acotadas, sin crecimiento sostenido y recuperación tras sleep/red |
| Privacidad | **0 conexiones salientes** en el escenario por defecto |
| Accesibilidad | recorrido completo con teclado y Narrator/NVDA sin función exclusiva de canvas |

Las métricas se medirán con hardware, versión del SO, permisos y configuración
registrados en `docs/VALIDATION.md`; no se compararán resultados de entornos distintos
como si fueran equivalentes.

## Línea 0 — Cerrar evidencia heredada de 0.3

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| BASE-01 | Matriz Windows normal/elevada y ETW real | P0 | REVIEW | — |
| BASE-02 | Ejecución nativa Linux y paquetes AppImage/deb | P0 | BLOCKED | — |
| BASE-03 | Narrator/NVDA y matriz completa HiDPI | P0 | REVIEW | — |
| BASE-04 | Bootstrap limpio e instalación/desinstalación | P0 | REVIEW | — |

### BASE-01 — Matriz Windows normal/elevada

- Ejecutar las fixtures TCP/UDP IPv4/IPv6 como usuario normal y en un contexto de
  trazado aprobado.
- Verificar IP Helper, ETW de red, DNS Client ETW, pérdida, reinicio de sesión y
  fallback tras `ACCESS_DENIED`.
- Separar el permiso necesario para captura avanzada del proceso de la interfaz.

**Aceptación:** el mismo informe compara capacidades normales y elevadas, conserva
fuente/limitación por campo y no atribuye contadores agregados a flujos individuales.

### BASE-02 — Linux nativo y paquetes

- Ejecutar la suite `INET_DIAG` en Ubuntu 22.04/24.04 y Debian 12.
- Cubrir IPv4/IPv6, TCP/UDP, `hidepid`, PID/inode, namespace visible, cierre durante
  muestreo y fallback sin capacidades especiales.
- Construir, lanzar, instalar y eliminar AppImage/deb sin alterar datos salvo elección
  explícita.

**Aceptación:** los paquetes se prueban en Linux real y `/proc/net` no reaparece como
fuente de sockets.

### BASE-03 — Accesibilidad e HiDPI

- Completar 1280×720, 1920×1080 y 2560×1440 a 100/125/150 %.
- Recorrer primer inicio, Live, Today, búsqueda, inspector, historial, ajustes y errores
  con teclado y Narrator/NVDA.
- Validar contraste, foco, zoom de texto, movimiento reducido y nombres accesibles.

**Aceptación:** no hay controles inaccesibles, solapes, scroll horizontal accidental ni
información disponible únicamente por color, hover, canvas o WebGL.

### BASE-04 — Reproducción e instalación limpia

- Instalar dependencias con lockfiles desde un clon limpio usando solo herramientas
  gratuitas.
- Ejecutar todos los gates, empaquetar, instalar, actualizar desde 0.3, desinstalar y
  verificar las opciones de conservar/eliminar datos.
- Confirmar que Tauri regenera esquemas ignorados y deja `git status --short` limpio.

**Aceptación:** un segundo entorno reproduce hashes de contenido equivalentes o deja
documentadas las únicas diferencias no deterministas del empaquetador.

## Línea A — Detección por eventos y fidelidad temporal

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| DET-01 | Harness local con verdad de referencia | P0 | DONE | BASE-01, BASE-02 |
| DET-02 | Contrato de capacidades, reloj y pérdida | P0 | DONE | DET-01 |
| DET-03 | Helper Windows mínimo y endurecido | P0 | REVIEW | BASE-01, DET-02 |
| DET-04 | Proveedor Windows realmente event-driven | P0 | REVIEW | DET-03 |
| DET-05 | Proveedor Linux eBPF opcional con fallback | P0 | BLOCKED | BASE-02, DET-02 |
| DET-06 | Fusión determinista de múltiples fuentes | P0 | DONE | DET-04, DET-05 |
| DET-07 | Ciclo de vida UDP y QUIC fiable | P0 | DONE | DET-06 |
| DET-08 | Supervisor y recuperación de proveedores | P1 | DONE | DET-02, DET-06 |

### DET-01 — Harness con verdad de referencia

- Crear un ejecutable de fixture que produzca un manifiesto de verdad con proceso,
  endpoints, protocolo, interfaz, apertura, actividad y cierre.
- Generar TCP/UDP IPv4/IPv6 de 5 ms, 20 ms, 50 ms, 100 ms, 1 s y larga duración;
  listeners, wildcard, UDP conectado/no conectado, reapertura de puerto y reutilización
  de PID.
- Simular ráfagas, pérdida, reorder, sleep, cambio de interfaz y cierre durante
  enumeración.
- Usar solo loopback/LAN controlada por defecto; Internet nunca será requisito del gate.

**Aceptación:** el harness compara verdad y eventos normalizados y produce recall,
precisión, latencia, falsos positivos y campos desconocidos por proveedor.

### DET-02 — Contrato de capacidades, reloj y pérdida

- Versionar capacidades como lifecycle por eventos, snapshot, bytes/paquetes, DNS,
  proceso, interfaz, ruta, namespace y metadatos de handshake.
- Normalizar reloj monotónico y UTC sin ordenar fuentes por wall-clock inseguro.
- Añadir secuencia por proveedor, watermark, gaps, drops y razón de degradación.
- Mostrar qué campo provino de qué capacidad, no solo el nombre genérico del backend.

**Aceptación:** cualquier pérdida o reinicio invalida únicamente las inferencias
afectadas y obliga a reconciliación explícita, nunca a una continuidad ficticia.

### DET-03 — Helper Windows mínimo y endurecido

- Crear un helper opcional para ETW/Pktmon con protocolo binario/JSON versionado y
  lista cerrada de operaciones.
- Usar named pipe con ACL, nonce de sesión, identidad del cliente y límites de tamaño,
  frecuencia y tiempo.
- No aceptar comandos, argumentos de shell, URLs ni rutas arbitrarias; una grabación
  recibe únicamente un destino ya validado mediante el protocolo previsto.
- Instalación, actualización, parada y eliminación son explícitas y reversibles.
- Añadir fuzzing del parser, tests de autorización y rechazo de versiones incompatibles.

**Aceptación:** la UI sigue sin privilegios; comprometer el webview no permite ejecutar
comandos ni leer/escribir archivos fuera de las operaciones cerradas del helper.

### DET-04 — Windows event-driven completo

- Capturar connect, accept, send, receive, disconnect, retransmit y DNS con esquemas ETW
  validados por versión de Windows.
- Resolver colisiones de nombres de sesión, reinicios del proveedor, wrap de contadores
  y eventos recibidos antes/después del snapshot.
- Conservar PID, proceso, 5-tupla, dirección, interfaz y timestamp cuando el evento los
  expone; marcar como ausente lo que el esquema no contiene.
- Usar IP Helper para reconciliar estado, no para sobrescribir evidencia temporal mejor.

**Aceptación:** cumple las métricas del harness y un fallo de ETW vuelve a IP Helper en
menos de dos segundos con aviso accionable y sin perder la sesión completa.

### DET-05 — Linux eBPF opcional

- Añadir un proveedor event-driven basado preferentemente en tracepoints/CO-RE para
  connect, accept, send/receive y close, sin módulo de kernel propio.
- Atribuir PID/TGID, UID, cgroup, namespace de red y proceso cuando estén disponibles.
- Detectar BTF/kernel/capacidades y volver a `INET_DIAG` automáticamente si no son
  suficientes.
- Documentar configuración mínima (`CAP_BPF`, `CAP_PERFMON` u opción equivalente por
  distro) sin ejecutar la UI como root.

**Aceptación:** Ubuntu/Debian cumplen las métricas del harness en modo eBPF y mantienen
captura honesta por `INET_DIAG` cuando eBPF está deshabilitado o no soportado.

### DET-06 — Fusión multi-fuente

- Correlacionar eventos, snapshots, DNS y contadores mediante identidad estable, reloj
  monotónico, interfaz y ventanas justificadas.
- Hacer idempotentes los reintentos y deduplicar sin sumar dos veces bytes/paquetes.
- Mantener candidatos cuando dos procesos o resoluciones son compatibles; no elegir uno
  arbitrariamente.
- Exponer `confirmed`, `correlated`, `candidate`, `snapshot_only` y `unknown` con razón.

**Aceptación:** reorder, duplicados, gaps y PID reuse no mezclan flujos ni historial; la
reproducción del mismo journal genera exactamente la misma proyección.

### DET-07 — UDP y QUIC

- Separar socket UDP enlazado, UDP conectado y peer observado por eventos.
- Definir identidad/expiración de conversación por datagramas sin inventar un cierre TCP.
- Reconocer QUIC únicamente por evidencia de handshake/version o como pista de puerto;
  ambas categorías se muestran distintas.
- Cubrir migración de conexión QUIC, cambio de puerto, IPv6 y reutilización rápida.

**Aceptación:** cero destinos falsos para UDP sin peer y las conversaciones QUIC
controladas sobreviven a cambios de ruta sin fusionarse con tráfico ajeno.

### DET-08 — Supervisor de proveedores

- Modelar estados `starting`, `healthy`, `degraded`, `restarting`, `fallback` y `failed`.
- Reiniciar workers con backoff acotado y circuit breaker; nunca crear loops agresivos.
- Reconciliar estado tras sleep, resume, cambio de red, helper actualizado o proceso
  proveedor terminado.
- Mantener captura disponible desde una fuente inferior cuando sea seguro.

**Aceptación:** pruebas de fallo inyectado recuperan cada subsistema sin bloquear UI,
SQLite, GeoIP o exportación y muestran desde cuándo se degradó.

## Línea B — Contexto causal de proceso, DNS y ruta

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| CTX-01 | Linaje e identidad local del ejecutable | P0 | REVIEW | DET-02 |
| CTX-02 | Ruta, interfaz, gateway y VPN | P0 | REVIEW | DET-06 |
| CTX-03 | Grafo causal DNS ↔ flujo | P0 | REVIEW | DET-06 |
| CTX-04 | Metadatos TLS/QUIC explícitos y efímeros | P1 | REVIEW | DET-03, DET-05, DET-07 |
| CTX-05 | Cambios de identidad del destino | P1 | DONE | CTX-01–CTX-04 |
| CTX-06 | Alcance de listeners y firewall local | P1 | REVIEW | CTX-02 |
| CTX-07 | Catálogos offline de puertos y sufijos | P2 | DONE | — |

### CTX-01 — Linaje e identidad del ejecutable

- Guardar instancia del padre, árbol de proceso, usuario/sesión y cambio exec/fork sin
  depender solo del PID.
- Leer Authenticode localmente en Windows y paquete/ELF/cgroup en Linux cuando exista.
- Calcular hash de ejecutable de forma lazy, cancelable y con presupuesto de I/O;
  invalidarlo si cambian tamaño, mtime o identidad de archivo.
- Presentar firma/hash como identidad local, nunca como reputación de seguridad.

**Aceptación:** una actualización/reemplazo del binario crea una revisión distinta del
perfil y un proceso hijo nunca hereda destinos como si fuera el padre.

### CTX-02 — Camino de red

- Capturar tabla de rutas, métrica, interfaz seleccionada, gateway y generación de red.
- Identificar adaptadores virtuales/túneles y distinguir ruta directa, VPN conocida,
  proxy del sistema y contexto desconocido.
- Guardar un perfil local pseudónimo; SSID, gateway y direcciones sensibles se redactan
  por defecto y el usuario puede poner un nombre propio.
- Diferenciar interfaz observada por el evento de ruta estimada por snapshot.

**Aceptación:** cambiar Wi-Fi/VPN/default route separa baselines y explica qué evidencia
confirma el camino y qué parte es solo una estimación temporal.

### CTX-03 — Grafo DNS causal

- Conservar query, CNAME, respuesta A/AAAA, TTL, resolver, proceso cuando esté disponible
  y ventana de validez.
- Relacionar un flujo con la resolución precedente usando proceso, IP, tiempo y TTL.
- Mantener múltiples candidatos para cachés compartidas, CDN, Happy Eyeballs y respuestas
  reutilizadas por varios procesos.
- Explicar DoH/DoT y caché previa como límites; no degradar una IP sin nombre.

**Aceptación:** la fixture DNS alcanza la precisión objetivo y ninguna resolución
ambigua se convierte en dominio confirmado.

### CTX-04 — Metadatos de handshake opcionales

- Añadir un modo separado, apagado por defecto, para extraer únicamente versión TLS,
  SNI, ALPN y versión/transport parameters QUIC cuando sean observables.
- Limitar bytes, paquetes, tiempo y protocolos; descartar buffers inmediatamente y no
  escribir payload ni claves en logs, SQLite o soporte ZIP.
- No implementar MITM, instalación de certificados, descifrado de sesión, HTTP body ni
  bypass de cifrado.
- Mostrar un indicador persistente mientras el modo esté activo y una auditoría local
  de inicio/parada.

**Aceptación:** fixtures locales TLS/QUIC demuestran extracción y borrado de buffers; un
test de privacidad confirma que no persisten contenidos ni se activa sin consentimiento.

### CTX-05 — Deriva de identidad

- Detectar cambios explicables de dominio → IP/prefijo/ASN/país, proceso → ejecutable y
  perfil de red → ruta.
- Comparar contra ventana y perfil de red equivalentes, no contra un baseline global.
- Separar cambio esperado, nuevo, frecuente y no evaluable; no generar un risk score.
- Permitir fijar el antes/después a una investigación.

**Aceptación:** el evento muestra exactamente qué campo cambió, primera/última vez,
fuentes y conexiones concretas que sustentan ambos lados.

### CTX-06 — Listeners y exposición local

- Mostrar bind loopback, LAN, VPN o wildcard, proceso/linaje, protocolo y primera/última
  observación.
- Consultar reglas relevantes del firewall mediante interfaces nativas de solo lectura
  del sistema operativo cuando sea viable; distinguir permitido, bloqueado, ambiguo y
  no consultable.
- Comparar listeners nuevos/eliminados tras instalar o actualizar software.
- No realizar escaneo remoto o de LAN automáticamente.

**Aceptación:** fixtures de listener más reglas controladas producen clasificación
correcta y nunca equiparan `0.0.0.0` con “accesible desde Internet” sin evidencia.

### CTX-07 — Catálogos offline

- Versionar snapshots revisados del registro de servicios/puertos IANA y Public Suffix
  List con licencia, fecha y hash.
- Usarlos como pista visual, no como detección de protocolo ni propietario.
- Actualizarlos solo al publicar una versión o mediante importación explícita.

**Aceptación:** la UI dice “servicio habitual para el puerto” y conserva siempre el
puerto/protocolo observado como dato primario.

## Línea C — Funciones nuevas útiles

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| FUN-01 | Vista “Today / Qué cambió” | P0 | DONE | CTX-05 |
| FUN-02 | Perfil histórico de aplicación | P0 | DONE | CTX-01, CTX-03 |
| FUN-03 | Perfiles de red y comparación | P1 | REVIEW | CTX-02 |
| FUN-04 | Motor local de reglas y etiquetas | P0 | DONE | FUN-01, FUN-02 |
| FUN-05 | Centro de notificaciones local | P1 | REVIEW | FUN-04 |
| FUN-06 | Auditoría de servicios expuestos | P1 | REVIEW | CTX-06 |
| FUN-07 | Paquete portable de investigación `.sabi` | P1 | DONE | FUN-01–FUN-04 |
| FUN-08 | Captura en bandeja y perfiles de operación | P1 | REVIEW | DET-08 |

### FUN-01 — Today / Qué cambió

- Crear una portada orientada a decisiones: nuevos procesos de red, destinos, dominios,
  ASNs, listeners, rutas/VPN y degradaciones desde la última sesión comparable.
- Separar “nuevo para este equipo”, “nuevo para esta aplicación” y “nuevo para este
  perfil de red”.
- Permitir abrir cada tarjeta en evidencia concreta y descartarla/fijarla localmente.
- Evitar rankings gigantes y porcentajes sin denominador.

**Aceptación:** desde Today se llega en máximo tres interacciones al proceso, flujo,
fuente y comparación que justifican cualquier cambio mostrado.

### FUN-02 — Perfil de aplicación

- Unificar ejecutable/revisión, linaje, destinos, dominios, puertos, protocolos, rutas,
  listeners, horas habituales y volumen histórico.
- Mostrar cambios entre versiones del binario sin mezclar revisiones.
- Añadir actividad reciente, primera/última observación y conexiones que respaldan cada
  agregado.
- Permitir etiquetar, filtrar, comparar y abrir en investigación.

**Aceptación:** los totales coinciden con la consulta común y cada cifra se puede
desglosar hasta eventos observados.

### FUN-03 — Perfiles de red

- Crear perfiles locales como Casa, Trabajo, VPN o Hotspot sin sincronización.
- Generar identidad pseudónima estable y permitir nombre manual; no mostrar SSID crudo
  en exportaciones por defecto.
- Comparar aplicaciones, destinos, listeners y rutas entre perfiles.
- Mantener baseline y reglas por perfil para reducir avisos irrelevantes.

**Aceptación:** cambiar de red no contamina el baseline anterior y una comparación
reconstruida conserva exactamente la ventana y perfil elegidos.

### FUN-04 — Reglas y etiquetas locales

- Añadir condiciones tipadas sobre proceso/revisión, dominio, IP/CIDR, ASN, país,
  protocolo, puerto, listener, perfil, ruta, primera vez, volumen y duración.
- Acciones permitidas: etiquetar, fijar, añadir a investigación y notificar.
- Incluir editor visual, vista previa sobre historial, contador de coincidencias y razón
  legible de cada disparo.
- Versionar reglas y detectar conflictos/duplicados.

**Aceptación:** el modo dry-run y la ejecución real coinciden en fixtures; ninguna regla
puede bloquear tráfico, lanzar shell o iniciar PCAP automáticamente.

### FUN-05 — Centro de notificaciones

- Bandeja local con agrupación, deduplicación, cooldown, quiet hours, snooze y estado
  leída/fijada/resuelta.
- Una notificación incluye regla, evidencia, perfil y acción concreta.
- Soportar notificación nativa opcional sin depender de un push service.
- No notificar continuamente si un proveedor está en fallback conocido.

**Aceptación:** una ráfaga de miles de eventos produce avisos acotados y cada aviso abre
el mismo conjunto de evidencia que lo originó.

### FUN-06 — Auditoría de exposición

- Vista dedicada a listeners actuales e históricos, cambio de bind, proceso, revisión,
  usuario, interfaz y evidencia de firewall.
- Destacar cambios desde una instalación/actualización o perfil de red sin calificarlos
  automáticamente como vulnerabilidad.
- Permitir snapshot firmado por hash, comparación y exportación redactada.

**Aceptación:** responde “qué escucha, desde cuándo y dónde podría ser accesible” sin
realizar escaneos ni afirmar alcance externo no comprobado.

### FUN-07 — Paquete `.sabi`

- Definir un contenedor ZIP versionado con manifiesto, consulta, ventanas, conexiones,
  timeline, notas, etiquetas, identidad de datasets y hashes de cada entrada.
- Ofrecer redacción de IP local, usuario, rutas y nombres sensibles antes de crear el
  archivo.
- Importar en modo de solo lectura sin mezclarlo con el historial local.
- Como P2 interno, evaluar cifrado con passphrase mediante una librería auditada; no
  diseñar criptografía propia.

**Aceptación:** exportar/importar reproduce la investigación y un byte alterado invalida
el manifiesto; el paquete nunca se abre o sube automáticamente.

### FUN-08 — Bandeja y perfiles de operación

- Añadir modos Visible, Background low-resource, Investigation high-fidelity y
  Ephemeral, con diferencias explicadas antes de activar.
- Bandeja con estado inequívoco, pausa, abrir, detener y salud; sin captura oculta.
- Autostart desactivado por defecto y configurable explícitamente.
- Aplicar límites distintos de frecuencia, retención, CPU y batería por perfil.

**Aceptación:** cerrar/minimizar nunca cambia silenciosamente el estado de captura; el
usuario puede saber desde el icono si se observa, persiste o usa helper/metadatos.

## Línea D — Experiencia de usuario e investigación

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| UX-01 | Nueva arquitectura Today / Live / Investigate / Exposure | P0 | DONE | FUN-01, FUN-06 |
| UX-02 | Onboarding de capacidades y permisos | P0 | REVIEW | DET-02, DET-03, DET-05 |
| UX-03 | Tabla de evidencia virtualizada | P0 | REVIEW | DET-06 |
| UX-04 | Constructor de consulta con autocompletado | P1 | REVIEW | FUN-04 |
| UX-05 | Workspace de investigación persistente | P0 | DONE | FUN-07 |
| UX-06 | Timeline reproducible y comparación visual | P1 | REVIEW | CTX-05 |
| UX-07 | Centro de salud y recuperación | P0 | REVIEW | DET-08 |
| UX-08 | Accesibilidad, escalado y personalización | P0 | REVIEW | BASE-03, UX-01–UX-07 |

### UX-01 — Arquitectura de información

- Convertir el globo/mapa en una herramienta dentro de Live, no en la única portada.
- Navegación principal: **Today**, **Live**, **Investigate**, **Exposure** y
  **Data & Health**.
- Mantener proceso/destino/selección al cambiar entre tabla, mapa, timeline y perfil.
- Reducir controles simultáneos y usar progressive disclosure para detalle técnico.

**Aceptación:** pruebas de tarea permiten encontrar cambio nuevo, flujo activo,
listener expuesto, investigación y estado del proveedor sin conocer la terminología
interna.

### UX-02 — Onboarding de capacidades

- Detectar primero lo que funciona sin privilegios y mostrar datos inmediatamente.
- Explicar beneficios/riesgos concretos antes de instalar helper o configurar eBPF.
- Ofrecer “Ahora no”, deshacer configuración y volver a comprobar.
- Evitar prompts genéricos de administrador y estados que parezcan error si son una
  limitación normal.

**Aceptación:** un usuario llega a Demo o captura básica en menos de tres minutos y
puede verificar después qué capacidades están activas y por qué.

### UX-03 — Tabla virtualizada y configurable

- Crear una vista de evidencia first-class para 100.000+ filas con columnas de proceso,
  destino, DNS, protocolo, ruta, estado, tiempo, calidad y fuente.
- Virtualizar sin romper lectores de pantalla: alternativa paginada/semántica y recuento
  total exacto.
- Permitir ordenar, agrupar, fijar columnas, ajustar ancho y guardar presets locales.
- Mantener scroll y selección estables mientras llegan deltas.

**Aceptación:** cumple la latencia objetivo, no salta al inicio con datos nuevos y la
misma consulta devuelve los mismos IDs en tabla, mapa y exportación.

### UX-04 — Constructor de consulta

- Autocompletar campos, operadores y valores observados sin enviar texto a servicios.
- Ofrecer chips editables, negación, grupos AND/OR, rango temporal y explicación de la
  consulta resultante.
- Round-trip exacto entre editor visual y gramática textual.
- Añadir historial local, presets y mensajes de error con posición y corrección.

**Aceptación:** property tests garantizan parse → editor → serialize sin cambiar
semántica y el teclado permite completar toda la consulta.

### UX-05 — Workspace de investigación

- Pestañas locales, breadcrumb causal, elementos fijados, notas, etiquetas y panel de
  evidencia que no desaparece al navegar.
- Guardado automático transaccional y recuperación tras cierre inesperado.
- Acciones claras para volver a Live, abrir perfil, comparar, exportar o cerrar.
- No duplicar conexiones en un store frontend independiente.

**Aceptación:** una investigación se reabre con consulta, rango, selección, layout y
notas; todas las relaciones siguen apuntando a IDs persistidos o al bundle importado.

### UX-06 — Timeline y comparación

- Scrubber con reproducción pausada, zoom, saltos a eventos y carriles para proceso,
  DNS, conexión, ruta, listener y degradación.
- Comparación lado a lado de dos intervalos/perfiles con escalas equivalentes.
- Mostrar datos faltantes/gaps, no interpolarlos como actividad real.
- Alternativa tabular completa y exportable.

**Aceptación:** mover el tiempo reconstruye el mismo estado que una consulta directa y
un gap del proveedor permanece visible en mapa, tabla y timeline.

### UX-07 — Data & Health

- Unificar captura, helper/eBPF, DNS, GeoIP, SQLite, colas, disco, batería y render.
- Cada degradación incluye impacto observable, detalle técnico y acción segura.
- Añadir restart/recheck/cancel/rollback cuando exista soporte real.
- Conservar historial local de cambios de estado para una investigación.

**Aceptación:** ningún error recuperable termina en spinner infinito o mensaje sin
acción; reiniciar un subsistema no borra la sesión ni oculta su gap.

### UX-08 — Accesibilidad y personalización

- Completar BASE-03 sobre la nueva navegación, tabla, timeline, reglas y notificaciones.
- Atajos configurables con detección de conflictos y botón de restaurar.
- Modos Compacto/Cómodo, texto ampliado, contraste reforzado y movimiento reducido.
- Restaurar layout únicamente cuando sigue siendo válido para el tamaño actual.

**Aceptación:** Narrator/NVDA y teclado completan los cinco recorridos principales; una
preferencia inválida o antigua migra a un valor seguro sin romper la UI.

## Línea E — Datos, fiabilidad, rendimiento y privacidad

| ID | Mejora | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| DATA-01 | Journal de evidencia y esquema SQLite v5 | P0 | DONE | DET-02 |
| DATA-02 | Proyecciones deterministas y replay | P0 | DONE | DATA-01, DET-06 |
| DATA-03 | Retención por niveles y búsqueda FTS | P1 | DONE | DATA-01 |
| DATA-04 | Backpressure extremo a extremo | P0 | DONE | DET-04, DET-05 |
| DATA-05 | Presupuestos de CPU, RAM, I/O y batería | P0 | REVIEW | FUN-08 |
| DATA-06 | Broker explícito de acciones de red | P0 | REVIEW | CTX-03, CTX-04 |
| DATA-07 | Recuperación, backup y salvage | P1 | DONE | DATA-01 |

### DATA-01 — Journal y SQLite v5

- Persistir observaciones inmutables con provider sequence, monotonic time, UTC,
  process/network generation y provenance; derivar conexiones como proyección.
- Separar evidencia observada, correlacionada, enriquecida y anotada por usuario.
- Migrar v1–v4 con backup, rollback, quick-check y compatibilidad de lectura.
- Indexar tiempo, proceso/revisión, IP/prefijo, dominio, ASN, ruta, listener y perfil.

**Aceptación:** ninguna re-enriquecimiento o corrección reescribe evidencia original y
una migración fallida deja base activa y backup legibles.

### DATA-02 — Replay determinista

- Reproducir journals con reloj virtual y provider gaps para generar conexiones,
  perfiles, cambios y notificaciones.
- Versionar reglas de proyección y permitir reconstrucción controlada.
- Comparar hash del resultado entre Windows/Linux para fixtures comunes.

**Aceptación:** mismo journal + versiones produce mismo conjunto ordenado de IDs,
totales y relaciones; diferencias de esquema se detectan antes de publicar.

### DATA-03 — Retención y búsqueda

- Retención distinta para evidencia detallada, conexiones, muestras y rollups.
- Mostrar estimación de espacio y efecto antes de aplicar; ejecutar por lotes cancelables.
- Añadir FTS5 para nombres, notas, etiquetas y detalle técnico, manteniendo filtros IP
  en índices tipados.
- Nunca compactar durante captura intensa sin presupuesto disponible.

**Aceptación:** borrar un nivel conserva los agregados prometidos, libera espacio tras
checkpoint/compact seguro y no deja referencias huérfanas.

### DATA-04 — Backpressure extremo a extremo

- Presupuesto separado para provider → fusion → storage → IPC → render.
- Priorizar lifecycle sobre métricas repetitivas y coalescer únicamente campos seguros.
- Medir profundidad, edad máxima, drops por clase y tiempo de recuperación.
- Forzar resync/proyección desde journal cuando un consumidor pierde secuencia.

**Aceptación:** una tormenta controlada no bloquea cierres/aperturas, mantiene memoria
acotada y explica exactamente qué clase de dato se descartó.

### DATA-05 — Presupuestos de recursos

- Definir perfiles low-resource/high-fidelity con cadencias y límites verificables.
- Pausar hashing, GeoIP histórico y mantenimiento bajo batería baja o presión de I/O.
- Instrumentar CPU/RAM/I/O por subsistema sin telemetría externa.
- Reducir detalle antes de perder lifecycle.

**Aceptación:** benchmarks y soak cumplen los objetivos o documentan un límite menor
por plataforma; cambiar perfil tiene efecto medible y visible.

### DATA-06 — Broker de red

- Centralizar toda acción iniciada por Sabicheiro: descarga completa GeoIP, DNS inverso
  opt-in y cualquier futura actualización de dataset.
- Requerir intención tipada, consentimiento aplicable, destino fijo, límites y log local
  redactado; ningún componente abre HTTP arbitrario.
- Añadir modo `offline enforced` que rechaza incluso acciones opcionales.
- Probar egress con proxy/servidor local y escenario sin red.

**Aceptación:** el escenario por defecto produce cero egress y cada conexión opcional
se atribuye a una acción visible; ninguna dirección observada se envía a un servicio
externo.

### DATA-07 — Recuperación y salvage

- Detectar cierre sucio, WAL inconsistente, disco lleno y corrupción parcial.
- Abrir en read-only, hacer backup y extraer filas válidas a una base nueva sin
  sobrescribir el original.
- Recuperar investigaciones/layout y marcar gaps que no puedan restaurarse.
- Permitir exportar diagnóstico redactado antes de resetear.

**Aceptación:** fault injection en cada fase no destruye la última base válida y todas
las acciones destructivas indican destino, backup y posibilidad de recuperación.

## Línea F — Validación y release 0.4.0

| ID | Gate | Prioridad | Estado | Dependencias |
| --- | --- | --- | --- | --- |
| QA-01 | Precisión y recall con verdad de referencia | P0 | REVIEW | DET-01–DET-07 |
| QA-02 | Matriz nativa Windows/Linux | P0 | BLOCKED | BASE-01, BASE-02, DET-03–DET-05 |
| QA-03 | Seguridad de helper, parser y bundle | P0 | DONE | DET-03, FUN-07 |
| QA-04 | Privacidad y egress | P0 | REVIEW | DATA-06 |
| QA-05 | Rendimiento, tormenta y soak de 8 h | P0 | REVIEW | DATA-04, DATA-05 |
| QA-06 | UX, accesibilidad y regresión visual | P0 | REVIEW | UX-01–UX-08 |
| QA-07 | Migración, actualización e instalación | P0 | REVIEW | BASE-04, DATA-01, DATA-07 |
| QA-08 | Release, SBOM, hashes y notas | P0 | REVIEW | QA-01–QA-07 |

### Cierre de ejecución del 14 de agosto de 2026

La ejecución de esta hoja de ruta queda auditada, no maquillada: **19 tareas `DONE`,
28 `REVIEW`, 3 `BLOCKED` y ninguna `TODO`/`IN_PROGRESS`**. El árbol construye y prueba
como candidato 0.4.0 en Windows, pero no cumple todavía el criterio de publicación
estable porque quedan P0 que exigen hardware/privilegios/tiempo externos y un proveedor
eBPF nativo que no se afirma implementado.

Evidencia común producida:

- `cargo fmt --all -- --check`, Clippy estricto y la suite completa: **125 pass,
  0 fail, 1 prueba ETW explícitamente ignorada** por requerir una sesión viva aprobada.
- Frontend de producción: TypeScript estricto, Vite y auditoría castellana de **839
  claves**; benchmark release de 100.000 filas con filtro p95 **21,3499 ms**.
- Harness IP Helper de 10 ms: precisión de destino/proceso **100 %**, **0 falsos
  destinos**, recall 5–50 ms **33,33 %**, recall ≥50 ms **66,67 %**, cierre p95
  **7,4386 ms**. El resultado demuestra el fallback, no las métricas event-driven.
- Auditoría offline: 519 paquetes Cargo y 42 pnpm sin licencias ausentes/prohibidas;
  SBOM SPDX 2.3 de 562 paquetes en `docs/sbom-0.4.0.spdx.json`.
- Release Windows y NSIS: build, arranque normal, instalación por usuario, arranque
  instalado y desinstalación pasaron; los datos locales se conservaron. Hashes exactos
  en `docs/SHA256SUMS-0.4.0.txt`.
- Detalle reproducible, entorno, comandos, métricas y bloqueos en
  `docs/VALIDATION.md`; notas en `docs/RELEASE_NOTES_0.4.0.md`.

#### Evidencia de tareas `DONE`

| ID | Evidencia que cierra la aceptación implementable |
| --- | --- |
| DET-01 | `harness.rs` genera verdad TCP/UDP IPv4/IPv6, duraciones, negativos y perturbaciones; compara precisión/recall/latencia/campos desconocidos y se ejecutó contra IP Helper. |
| DET-02 | `evidence.rs` versiona capacidades, reloj, cursor, watermark, gaps, drops, calidad y procedencia por campo; tests invalidan solo capacidades afectadas. |
| DET-06 | `fusion.rs` deduplica de forma idempotente, conserva candidatos/gaps y prueba reorder, duplicados, contadores, PID reuse y proyección canónica. |
| DET-07 | `udp.rs` separa bind/peer/conversación, expira sin inventar cierre y prueba QUIC por handshake frente a pista, IPv6 y migración de ruta. |
| DET-08 | `supervisor.rs` prueba estados, backoff, circuit breaker, fallback y reconciliación tras cambio de entorno sin continuidad ficticia. |
| CTX-05 | Comparación tipada de revisiones/dominios/prefijos/ASN/país/ruta respeta perfil de red y Today permite fijar antes/después hasta IDs de evidencia. |
| CTX-07 | Snapshots IANA/PSL fechados, licenciados y con SHA-256; catálogo/test/UI dicen pista habitual y preservan puerto/protocolo observado. |
| FUN-01 | Today calcula cambios contra sesiones comparables, excluye la actual, persiste fijado/descartado y abre la evidencia en una acción. |
| FUN-02 | Perfil histórico comparte los IDs de la consulta y desglosa revisiones, destinos, DNS, puertos, rutas, listeners, horas, volumen y primera/última vez. |
| FUN-04 | Condiciones/acciones tipadas, editor visual, dry-run de 100.000 filas, versiones y conflictos; dry-run/ejecución comparten evaluador y ninguna acción peligrosa existe. |
| FUN-07 | `.sabi` tiene conjunto exacto y acotado, CRC + hashes de manifiesto, redacción, importación aislada read-only y test de byte alterado. |
| UX-01 | Navegación primaria exacta Today/Live/Investigate/Exposure/Data & Health; mapa/globo viven dentro de Live y selección/filtro son comunes. |
| UX-05 | Workspace conserva consulta, ventana, selección, layout, notas/tags y autosave transaccional; la reapertura reconstruye desde IDs persistidos o bundle aislado. |
| DATA-01 | SQLite v5 separa journal inmutable, proyección, enriquecimiento/anotación y migra todas las versiones con backup/rollback/quick-check. |
| DATA-02 | Replay con versión/hash de proyección prueba idempotencia, cursor seguro y salida determinista ante reorder/gaps. |
| DATA-03 | Retención por niveles estima, agrega y borra por lotes; FTS local e índices tipados tienen tests de referencias y huérfanos. |
| DATA-04 | Colas provider/fusion/storage/IPC/render son acotadas, priorizan lifecycle, coalescen solo claves seguras y fuerzan resync al perder secuencia. |
| DATA-07 | Tests de cierre/migración fallida, backup y salvage crean destino nuevo, conservan original legible y nunca lo sobrescriben. |
| QA-03 | Parser/helper soporta bytes arbitrarios sin panic/asignación ilimitada, autentica PID/nonce y limita tamaño/tasa/tiempo; `.sabi` detecta manipulación. |

#### Tareas `REVIEW` y evidencia exacta que falta

| ID | Lo implementado | Evidencia que impide `DONE` |
| --- | --- | --- |
| BASE-01, DET-03, DET-04, QA-01 | IP Helper, ETW real, helper cerrado, fallback y harness existen. | El usuario normal recibe `ACCESS_DENIED`; falta ejecutar ETW/helper aprobado, pérdida/reinicio y cumplir recall event-driven. |
| BASE-03, UX-08, QA-06 | Semántica, teclado, alternativa tabular, reduced motion, contraste reforzado y escala de texto están implementados. | El navegador quedó bloqueado por su política al volver desde su página interna de error; faltan 3 resoluciones × 3 escalas y Narrator/NVDA reales. |
| BASE-04, QA-07 | Lockfiles, migración v1–v5, release/NSIS y ciclo instalar/lanzar/desinstalar Windows pasan. | Falta clon/actualización 0.3 y ciclo AppImage/deb en hosts limpios; el bootstrap offline evidenció un tarball ausente antes de restaurar desde caché. |
| CTX-01 | Linaje, usuario/sesión, revisión de fichero, hash lazy cancelable y Authenticode/ELF-paquete están modelados y visibles. | Falta fixture nativa Linux y reemplazo de binario firmado/paquete para validar todas las ramas. |
| CTX-02 | `GetBestRoute2`, comando Linux fijo, interfaz/gateway/métrica/calidad y perfil pseudónimo están implementados. | Falta matriz nativa Wi-Fi/VPN/ruta por defecto y prueba Linux; conexiones v5 heredadas no reciben un perfil retroactivo inventado. |
| CTX-03 | Grafo DNS conserva TTL/CNAME/resolver/proceso y mantiene confirmado/candidatos con límites DoH/caché. | Falta fixture ETW/eBPF viva que mida precisión ≥99 % con CDN/Happy Eyeballs/caché compartida. |
| CTX-04 | Parser ClientHello, buffer ceroizado y acotado, consentimiento/indicador/auditoría existen. | Falta cableado a eventos vivos y fixtures TLS/QUIC que prueben límites de bytes/paquetes/tiempo y escaneo de no persistencia. |
| CTX-06, FUN-06 | Exposure clasifica bind, proceso/revisión/usuario/interfaz, snapshots hash, comparación y exportación redactada sin escaneo. | Firewall se declara honestamente `not_queryable`; faltan adaptadores nativos read-only y fixtures de reglas. |
| FUN-03 | Perfiles pseudónimos estables, nombre local y comparación VPN/ruta están disponibles. | Falta asignación de perfil a nuevas filas de conexión y cambio real de red para demostrar baseline/reglas aislados extremo a extremo. |
| FUN-05 | Inbox local agrupa/deduplica, aplica cooldown/quiet hours/snooze/estado y abre IDs originales. | Falta notificación nativa opcional y prueba visual/tormenta desde el runtime completo. |
| FUN-08 | Tray muestra captura/persistencia/helper/metadatos y ofrece abrir/pausar vista/detener/salir; autostart es opt-in y los perfiles cambian cadencias. | Falta política medible de batería/I/O y matriz nativa de cerrar/minimizar por SO. |
| UX-02 | La UI enumera capacidades/fallbacks, ofrece recheck y no exige privilegio para Demo/IP Helper. | No puede cerrar onboarding de helper/eBPF hasta que DET-03/05 tengan ciclo nativo. |
| UX-03 | Tabla virtualizada, sort/grupo/ancho/columnas/presets y alternativa semántica paginada están implementados. | Falta medición de interacción en webview con 100.000 filas y estabilidad visual de scroll/selección bajo deltas. |
| UX-04 | Gramática Rust/TS con AND/OR/NOT/grupos, autocompletado local, chips, errores posicionales, historial y round-trip tiene tests. | Falta property/fuzz de mayor amplitud y recorrido completo solo con teclado en webview nativa. |
| UX-06 | Timeline/reconstrucción/comparación y tablas exportables preexistentes consumen consulta común. | Falta revisión visual del scrubber/carriles y prueba de gaps coherentes simultáneamente en mapa/tabla/timeline. |
| UX-07 | Data & Health reúne proveedores, colas, SQLite, GeoIP, recursos y acciones soportadas, con historial persistido. | Falta matriz completa de restart/recheck/cancel/rollback y batería/render en aplicación viva. |
| DATA-05, QA-05 | Perfiles, presupuestos/colas, tormenta y benchmark 100k pasan; soak corto: 311.296 B de crecimiento. | Faltan ocho horas de captura de escritorio, sleep/red y políticas de batería/I/O en hardware registrado. |
| DATA-06, QA-04 | Broker tipado, consentimiento, destino fijo, límites, auditoría y `offline_enforced` tienen tests; default no concede egress. | Falta captura externa mediante proxy/servidor local que demuestre cero paquetes de proceso en el escenario por defecto. |
| QA-08 | 0.4.0 sincronizado, NSIS/PE, SBOM, SHA-256, changelog, notas y validación coinciden. | Es un candidato: regenerar artefactos de ambos SO solo después de cerrar todos los P0 anteriores. |

#### Bloqueos externos reproducibles

| ID | Bloqueo observado | Siguiente acción obligatoria |
| --- | --- | --- |
| BASE-02 | No hay WSL, Docker, Podman ni host Linux; el cross-check ampliado llega a `ring` y no encuentra `x86_64-linux-gnu-gcc`. | Ejecutar suite/fixtures/paquetes en Ubuntu 22.04/24.04 y Debian 12 reales, incluyendo hidepid/namespaces/IPv6/instalar/eliminar. |
| DET-05 | El contrato/fallback eBPF existe, pero este candidato no contiene un programa eBPF de producción y no hay kernel Linux/BTF para desarrollarlo/validarlo con honestidad. | Implementar tracepoints/CO-RE en Linux, documentar capacidades mínimas y superar el harness con y sin permisos. |
| QA-02 | La matriz exige simultáneamente Windows normal/aprobado y Linux INET_DIAG/eBPF; solo Windows normal y source-check Linux están disponibles. | Repetir los mismos hashes/fixtures en los entornos nativos anteriores y adjuntar informes comparables. |

### Gates obligatorios

1. `cargo fmt --all -- --check`.
2. `cargo clippy --workspace --all-targets --locked -- -D warnings`.
3. `cargo test --workspace --all-targets --locked` en Windows y Linux.
4. Build TypeScript/Vite y auditoría i18n con lockfile.
5. Harness de verdad con informe de precisión, recall y latencia por proveedor/modo.
6. Windows normal + helper aprobado; Linux `INET_DIAG` + eBPF + fallback sin capacidad.
7. Fuzzing de correlación, protocolo del helper, import `.sabi` y parsers TLS/QUIC.
8. Auditoría offline de dependencias/licencias y SBOM versionado.
9. Prueba de egress: cero red por defecto y únicamente acciones explícitas permitidas.
10. Tormenta reproducible, 100.000 filas históricas y soak de ocho horas.
11. Matriz visual/teclado/Narrator/NVDA a 100/125/150 % y movimiento reducido.
12. Migración real desde 0.3, rollback, disco lleno, corrupción y recuperación.
13. Portable/NSIS y AppImage/deb: launch, update, install, uninstall y datos retenidos.
14. SHA-256, notas de release, changelog y evidencia final en `docs/VALIDATION.md`.

## Orden de implementación

1. **Cerrar la base:** BASE-01–BASE-04 y DET-01–DET-02.
2. **Captura por eventos:** DET-03–DET-08 y DATA-04.
3. **Persistencia reproducible:** DATA-01–DATA-03 y DATA-07.
4. **Contexto causal:** CTX-01–CTX-07.
5. **Funciones útiles:** FUN-01–FUN-08.
6. **Nueva experiencia:** UX-01–UX-08.
7. **Privacidad y recursos:** DATA-05–DATA-06.
8. **Cierre:** QA-01–QA-08 y release 0.4.0.

## Criterio de salida

La 0.4.0 no se publica como release estable mientras:

- exista una tarea P0 en `TODO`, `IN_PROGRESS`, `BLOCKED` o `REVIEW`;
- los proveedores por eventos no alcancen las métricas acordadas en sus entornos;
- un helper o eBPF sea requisito para obtener cualquier dato útil;
- haya una relación proceso/DNS/ruta presentada como confirmada sin evidencia;
- el modo por defecto produzca una conexión saliente;
- una función principal dependa de cuenta, cuota, API o servicio de pago;
- la tabla/timeline no soporte el volumen objetivo o pierda selección con deltas;
- Narrator/NVDA no pueda completar los recorridos principales;
- migración, recuperación e instalación/desinstalación no tengan evidencia nativa;
- los paquetes, hashes, SBOM, privacidad y notas no coincidan con el producto.

## Fuera de alcance

- Antivirus, reputación cloud, feed de amenazas o clasificación de malware.
- Bloquear conexiones, editar firewall o sustituir un EDR/firewall.
- Escanear Internet o LAN automáticamente.
- Inspeccionar payload por defecto, hacer MITM o descifrar sesiones TLS.
- Subir PCAP, investigaciones, IP, dominios, hashes o telemetría.
- API GeoIP por dirección observada, mapas remotos o tiles de terceros.
- Cuenta, sincronización entre dispositivos o colaboración cloud.
- Driver de kernel propio en Windows.
- Aplicaciones móviles o soporte macOS.
