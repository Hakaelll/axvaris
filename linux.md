# Instalar y usar Sabicheiro en Linux

Esta guía está pensada para **Ubuntu 24.04 o Debian 12 en x86_64**. Compila la aplicación desde el código fuente y genera un paquete `.deb` o un AppImage. La captura normal de Linux usa `INET_DIAG` para observar los sockets de este ordenador y relacionarlos con procesos cuando el sistema lo permite. No requiere ejecutar la interfaz como `root` ni instalar eBPF. La grabación de paquetes es una función aparte.

> Estado de validación: el proyecto se ha compilado y probado en Windows. Los comandos y dependencias Linux se han revisado contra la configuración del proyecto y la documentación de Tauri, pero esta revisión no dispone de un host Linux para ejecutar el paquete final. Comprueba la instalación en tu distribución antes de darla por validada.

## 1. Instalar las dependencias del sistema

Abre una terminal y copia este bloque completo:

```bash
sudo apt update
sudo apt install -y git curl wget file build-essential pkg-config \
  libwebkit2gtk-4.1-dev libxdo-dev libssl-dev \
  libayatana-appindicator3-dev librsvg2-dev patchelf iproute2
```

WebKitGTK y el indicador son requisitos de Tauri; `iproute2` proporciona `ip`, que Sabicheiro usa para consultar interfaces, rutas y la caché de vecinos de la red local. No hace falta `dumpcap` para la observación normal de conexiones.

## 2. Instalar Rust, Node.js y pnpm

Instala Rust con `rustup` y activa sus ejecutables en la terminal actual:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
. "$HOME/.cargo/env"
rustup component add rustfmt clippy
```

Instala Node.js 24 mediante `nvm` y la versión de pnpm fijada por el proyecto:

```bash
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.8/install.sh | bash
export NVM_DIR="$HOME/.nvm"
. "$NVM_DIR/nvm.sh"
nvm install 24
nvm use 24
npm install --global pnpm@11.16.0
```

Comprueba las herramientas:

```bash
rustc --version
cargo --version
node --version
pnpm --version
ip -V
pkg-config --modversion webkit2gtk-4.1
```

Si abres una terminal nueva y `nvm` o `cargo` aún no aparecen, vuelve a cargar tu sesión de shell o ejecuta `. "$HOME/.cargo/env"` y `. "$HOME/.nvm/nvm.sh"`.

## 3. Descargar y comprobar el proyecto

```bash
git clone https://github.com/Hakaelll/sabicheiro.git
cd sabicheiro
pnpm install --frozen-lockfile
pnpm --dir apps/desktop check
cargo test --workspace --all-targets --locked
```

La primera compilación de Rust descarga y compila dependencias; puede tardar varios minutos. `pnpm install` usa las versiones fijadas en `pnpm-lock.yaml`. Si una comprobación falla, conserva la salida completa de la terminal para diagnosticarla.

## 4. Abrir la aplicación

Para desarrollo, con la terminal situada en la carpeta `sabicheiro`:

```bash
pnpm tauri dev
```

Esto abre la aplicación nativa. `pnpm dev` solo abre una vista previa web y **no** permite validar la captura real de sockets.

Para crear los paquetes de instalación:

```bash
pnpm tauri build --bundles deb,appimage
```

### Instalar el paquete Debian/Ubuntu

```bash
DEB_FILE="$(find target/release/bundle/deb -maxdepth 1 -type f -name '*.deb' -print -quit)"
if [ -n "$DEB_FILE" ]; then
  sudo apt install "$PWD/$DEB_FILE"
else
  echo 'No se ha generado el paquete .deb'
fi
```

Después, abre **Sabicheiro** desde el menú de aplicaciones. `apt` instalará las dependencias declaradas del paquete. Para desinstalarlo sin borrar manualmente sus datos de usuario:

```bash
sudo apt remove sabicheiro
```

### Ejecutar el AppImage

```bash
APPIMAGE_FILE="$(find target/release/bundle/appimage -maxdepth 1 -type f -name '*.AppImage' -print -quit)"
if [ -n "$APPIMAGE_FILE" ]; then
  chmod +x "$APPIMAGE_FILE"
  "$APPIMAGE_FILE"
else
  echo 'No se ha generado el AppImage'
fi
```

Si el AppImage indica que falta FUSE, instala el paquete de compatibilidad de tu distribución (`libfuse2t64` en Ubuntu 24.04; `libfuse2` en Debian 12) o usa el `.deb`. No ejecutes Sabicheiro con `sudo`.

## 5. Activar las ubicaciones de conexiones públicas

La ubicación aproximada necesita una base **City MMDB local**. La aplicación funciona sin ella, pero entonces las conexiones públicas aparecen sin ubicación en el mapa. Para instalarla:

1. Abre **Datos y estado → GeoIP**.
2. Pulsa **Instalar base City gratuita**. La aplicación descarga la base DB-IP Lite completa desde su origen fijo; no envía las IP observadas para consultarlas.
3. Comprueba el estado de la base en la misma pantalla. También puedes importar un archivo City MMDB que ya tengas. La base ASN es opcional y añade información sobre el operador de red.

Al instalar, importar, sustituir, restaurar o quitar una base, las conexiones visibles se recalculan. Las ubicaciones son aproximadas; el dato puede faltar si no hay coincidencia en la base. Las IP privadas o de loopback **no tienen una ubicación geográfica pública**: consulta la pestaña **Dispositivos** para ver las direcciones y conexiones locales. Las direcciones IPv4 e IPv6 se agrupan como un mismo equipo solo cuando la caché de vecinos del sistema proporciona la misma MAC e interfaz. Los demás equipos se muestran como direcciones sin identidad verificada. La aplicación no escanea toda la red local.

## 6. Grabación opcional de paquetes

La observación normal de sockets no necesita privilegios especiales. Si quieres usar la función explícita de grabación PCAPNG, instala `dumpcap` y habilita el grupo de captura según la política de Debian/Ubuntu:

```bash
sudo apt install -y wireshark-common
sudo dpkg-reconfigure wireshark-common
sudo usermod -aG wireshark "$USER"
```

En la pregunta del configurador, permite la captura a usuarios del grupo `wireshark`. **Cierra la sesión de escritorio y vuelve a entrar** para activar la pertenencia al grupo. Después verifica:

```bash
groups
dumpcap -D
```

No des permisos amplios a la aplicación ni la ejecutes como administrador. La grabación de paquetes puede incluir contenido sensible: actívala únicamente cuando la necesites y en una red que administras o donde tengas autorización.

## 7. Qué puede mostrar sobre otros equipos

Sabicheiro observa el tráfico de **este ordenador** y consulta la caché de vecinos del sistema. En una red Ethernet o Wi-Fi conmutada, eso permite ver algunos equipos locales con los que este ordenador se ha comunicado, incluso si ya no hay un socket activo. La caché no enumera todos los aparatos conectados al router ni permite ver todas sus conexiones o búsquedas. Los nombres mostrados deben proceder de observaciones disponibles; no se inventan nombres, fabricantes ni ubicaciones.

Para conocer los dominios consultados por **otros** equipos se necesita una fuente adicional, como los registros DNS del router o de un servidor DNS que utilicen esos equipos. Una conexión HTTPS normal no revela el texto de una búsqueda ni el contenido de una página. Esa función requiere una integración específica y una fuente de datos configurada; la instalación básica no la incluye.

La preparación y comprobación de esa fuente se explica en [dns.md](dns.md).

## 8. Diagnóstico rápido

```bash
cd sabicheiro
pnpm --dir apps/desktop check
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --all-targets --locked
ip address
ip route
```

- **No aparecen ubicaciones:** comprueba que la base City está instalada en **Datos y estado → GeoIP** y que la conexión es pública. La pestaña **Dispositivos** representa equipos locales sin asignarles coordenadas falsas.
- **Falta un dispositivo local:** comprueba `ip -j neigh show`. Si el sistema no conoce su dirección, Sabicheiro tampoco puede identificarlo por MAC. Comunícate con ese equipo desde este ordenador y actualiza la vista.
- **Faltan procesos o interfaces:** comprueba que `iproute2` está instalado. Algunas conexiones de otros usuarios, espacios de nombres de red, VPN o contenedores pueden limitar la atribución disponible para el usuario actual.
- **El globo no aparece:** comprueba el soporte WebGL del sistema o usa el mapa 2D. La vista de equipos locales funciona sin WebGL.
- **`dumpcap -D` falla:** vuelve a iniciar sesión después de añadir el usuario al grupo y revisa la configuración de `wireshark-common`.
- **Falla la construcción de Tauri:** verifica que `pkg-config --modversion webkit2gtk-4.1` responde y que usas Ubuntu/Debian x86_64 con las dependencias de la sección 1.

Referencias: [requisitos de Tauri para Linux](https://v2.tauri.app/start/prerequisites/), [paquetes Debian de Tauri](https://v2.tauri.app/distribute/debian/), [permisos de captura de Wireshark](https://wiki.wireshark.org/capturesetup/captureprivileges/) y [FUSE para AppImage](https://docs.appimage.org/user-guide/troubleshooting/fuse.html).
