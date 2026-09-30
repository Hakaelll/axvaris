# Preparar la consulta DNS por equipo de la red

Este documento explica cómo obtener **dominios consultados por los dispositivos de tu red** usando un router o servidor DNS que administras. **Sabicheiro todavía no importa ni muestra registros DNS de otros equipos**: la vista **Dispositivos** combina conexiones observadas desde este ordenador con entradas de la caché de vecinos del sistema, sin atribuir consultas DNS ajenas. Esta guía permite preparar y verificar una fuente de datos para una integración posterior.

## Qué información se obtiene

Un registro DNS útil debe contener, al menos, **fecha y hora, dirección IP del cliente, nombre de dominio consultado y resultado**. Un dominio solicitado indica una consulta de resolución; no confirma que una persona haya visitado la página, leído su contenido ni qué términos haya escrito en un buscador. Una aplicación puede consultar dominios en segundo plano. Las consultas servidas desde caché local, DNS cifrado externo (DoH/DoT), una VPN o un resolver distinto pueden no aparecer.

Para atribuir consultas a un equipo, el servidor DNS debe recibirlas **directamente con la IP de ese equipo**. Si el router reenvía todas las consultas en su propio nombre, el registro puede mostrar únicamente la IP del router. Vincula la IP con las concesiones DHCP del momento; una IP puede cambiar o reutilizarse y no identifica permanentemente a un dispositivo.

## Preparar la red

1. Elige un servidor DNS que registre consultas por cliente: el DNS del router, Pi-hole o AdGuard Home.
2. Configura el DHCP de la red para entregar **la dirección del servidor elegido** como DNS a los equipos. Si el DNS está en otra máquina, usa su IP local, no la dirección pública de un proveedor. Renueva la concesión DHCP o reconecta cada equipo.
3. En el servidor DNS, activa el registro de consultas y comprueba que conserva la IP cliente sin anonimizarla. Ajusta el tiempo de retención y limita quién puede leer el registro.
4. Desde **uno de los equipos que quieres observar**, haz una consulta de prueba. Sustituye `192.168.1.1` por la IP real de tu servidor DNS:

   ```bash
   nslookup example.org 192.168.1.1
   ```

5. Busca `example.org` en el registro del DNS. Comprueba que la IP cliente corresponde al equipo de prueba y que la hora es correcta. Repite la prueba desde otro equipo. Si ambos aparecen como el router, revisa cómo se anuncia o reenvía el DNS.

La consulta con `nslookup` verifica que ese servidor responde y registra una prueba puntual. Para confirmar que es el DNS **habitual** del dispositivo, revisa también la configuración DNS recibida por DHCP en ese dispositivo.

## Opción A: OpenWrt con dnsmasq

Si tu router usa OpenWrt y dnsmasq, entra por SSH y activa el registro de consultas. Estos comandos son para **ejecutar en el router**, no en el ordenador con Sabicheiro:

```sh
uci set dhcp.@dnsmasq[0].logqueries='1'
uci commit dhcp
service dnsmasq restart
logread -f | grep dnsmasq
```

Con `logread -f` abierto, haz la consulta de prueba desde un cliente. Debe aparecer el dominio y la IP cliente. Pulsa `Ctrl+C` para dejar de seguir el registro. El registro por defecto de OpenWrt suele estar en memoria y puede rotar o perderse al reiniciar; si necesitas conservarlo, configura un destino syslog o almacenamiento con una política de retención adecuada. Para desactivar el registro:

```sh
uci set dhcp.@dnsmasq[0].logqueries='0'
uci commit dhcp
service dnsmasq restart
```

El ajuste `logqueries` y el uso de `logread` están documentados por [OpenWrt: configuración DNS y DHCP](https://openwrt.org/docs/guide-user/base-system/dhcp_configuration) y [OpenWrt: registros](https://openwrt.org/docs/guide-user/base-system/log.essentials).

## Opción B: Pi-hole

En la interfaz de administración, comprueba que el registro de consultas está habilitado y abre **Query Log**. Filtra por la IP del cliente de prueba y busca `example.org`. Comprueba el nivel de privacidad configurado: algunos niveles ocultan el dominio o el cliente. Pi-hole documenta los [niveles de privacidad](https://docs.pi-hole.net/ftldns/privacylevels/) y la [base de consultas](https://docs.pi-hole.net/database/query-database/), donde cada consulta puede vincularse con un cliente.

Si solo aparece el router, configura el DHCP para anunciar la IP de Pi-hole directamente a los clientes y repite la prueba. No copies la base de datos activa de Pi-hole para abrirla a mano desde Sabicheiro; esa integración no existe aún y se deberá diseñar para evitar lecturas incompletas o problemas de permisos.

## Opción C: AdGuard Home

Activa **Query Log** en los ajustes de AdGuard Home y verifica que el registro no anonimiza la IP cliente. En la vista de consultas, filtra por la IP del equipo de prueba y busca `example.org`. AdGuard Home documenta las opciones [`querylog_enabled`, `querylog_file_enabled` y `anonymize_client_ip`](https://github.com/AdguardTeam/AdGuardHome/wiki/Configuration). Sus [ajustes de clientes](https://github.com/AdguardTeam/AdGuardHome/wiki/Clients) permiten asignar nombres descriptivos, pero esos nombres dependen de la configuración y de la información observada.

## Datos que necesitaría Sabicheiro para una integración futura

Para mostrar consultas por equipo sin adivinar su procedencia, haría falta elegir una fuente concreta y documentar su formato o API. Cada registro importado debería conservar `timestamp`, `client_ip`, `query_name`, `query_type`, `result` y `source`. La interfaz tendría que separar claramente **consultas DNS del router** de **conexiones del ordenador local** y mostrar cuándo el cliente es desconocido, una IP compartida o una atribución basada solo en DHCP.

Hasta que esa función se implemente, consulta los dominios en la interfaz del propio router, Pi-hole o AdGuard Home. No pegues registros DNS reales en incidencias públicas: pueden revelar hábitos de los usuarios de la red.
