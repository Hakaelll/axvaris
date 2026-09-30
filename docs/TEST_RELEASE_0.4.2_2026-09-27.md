# Sabicheiro 0.4.2 — paquete de prueba del 27 de septiembre de 2026

Este paquete de prueba incorpora las correcciones posteriores al primer instalador 0.4.2. La compilación actual se entrega en `release/Sabicheiro-0.4.2-test3-windows-x64/`, con hashes propios para distinguirla de test2. El código fuente está en el árbol de trabajo actual; aún no se ha publicado una etiqueta ni una versión de GitHub.

## Cambios relevantes

- Las capturas de sockets ya no envían una actualización por cada conexión sin cambios en cada sondeo. Se mantiene un pulso cada 10 segundos para las conexiones estables.
- La instalación de DB-IP City Lite funciona aunque falle la descarga opcional de ASN. Las conexiones públicas visibles se vuelven a localizar al instalar, importar, restaurar o retirar la base.
- El mapa y el inspector distinguen una base City ausente, una dirección local sin geolocalización pública y una dirección sin coincidencia. El mapa y el inspector ofrecen la instalación de City desde el propio contexto.
- La vista **Dispositivos** añade la caché de vecinos del sistema sin hacer escaneos de red. Agrupa IPv4 e IPv6 solo si coinciden la MAC y la interfaz, separa las IP sin identidad verificada y muestra conexiones, procesos y puertos observados cuando existen.
- El mapa 2D usa menos elementos SVG para las fronteras. El globo evita reasignaciones de resolución de GPU en cada actualización y cambia al mapa 2D si WebGL falla, pierde el contexto o su componente lanza un error.
- La interfaz reduce trabajo cuando está oculta y evita solapar ciertas consultas periódicas. La consulta DNS de otros equipos queda pendiente; `dns.md` explica la preparación de una fuente DNS futura.

## Comprobaciones realizadas

- `pnpm --dir apps/desktop check`: auditoría de 1026 claves traducidas, pruebas de comportamiento, TypeScript y Vite correctos.
- `cargo test --workspace --locked --offline`: pruebas superadas y una prueba ETW ignorada porque exige una sesión Windows autorizada.
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` y `cargo fmt --all -- --check`: correctos.
- Auditorías de estructura, frontend, dependencias directas y licencias: correctas.
- Compilación nativa de Windows y generación del instalador NSIS: correctas.

Estas comprobaciones no reproducen aún una sesión interactiva larga con el equipo y la base de datos del usuario. La corrección del bloqueo debe confirmarse usando la aplicación instalada durante varios minutos.

## Prueba recomendada

1. Cierra el Sabicheiro anterior e instala `Sabicheiro_0.4.2_x64-setup.exe` desde la carpeta del paquete de prueba.
2. Abre la aplicación, inicia **Captura en vivo** y úsala durante al menos cinco minutos, alternando entre Remoto 3D, Remoto 2D, Dispositivos, Hoy e Inspector.
3. En Remoto, pulsa **Instalar base City gratuita**. Es una descarga explícita y grande; necesita conexión a Internet. Espera a que termine y comprueba que se muestran las ubicaciones de las IP públicas con coincidencia. Las IP privadas y de loopback se muestran en Local y no tienen ubicación geográfica pública.
4. Si vuelve a dejar de responder, conserva la hora aproximada y utiliza **Datos y estado → Diagnóstico → Exportar paquete de soporte** para facilitar el análisis. No borres la base de datos antes de ello.

Los hashes SHA-256 de los ejecutables figuran en `SHA256SUMS.txt` dentro del paquete. El instalador no está firmado digitalmente.
