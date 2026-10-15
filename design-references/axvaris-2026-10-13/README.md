# Referencias visuales de Axvaris

Este conjunto contiene 13 propuestas de pantalla generadas con la herramienta integrada de imágenes. Conserva la identidad oscura de Axvaris y explora una composición más visual, inspirada en herramientas de análisis como Gotham.

Abre [la galería](index.html) para ampliar cada imagen y consultar su original. Los PNG de esta carpeta son las referencias seleccionadas. `originales/` conserva las 13 capturas de entrada. `variantes/` conserva las primeras propuestas que se corrigieron. Los prompts completos están en [prompts.json](prompts.json).

## Pantallas

| Captura | Referencia |
| --- | --- |
| Hoy · Resumen | [01-hoy-resumen.png](01-hoy-resumen.png) |
| Hoy · Comparación y reglas | [02-hoy-comparacion-reglas.png](02-hoy-comparacion-reglas.png) |
| En vivo · Globo 3D | [03-en-vivo-globo-3d.png](03-en-vivo-globo-3d.png) |
| En vivo · Dispositivos | [04-en-vivo-dispositivos.png](04-en-vivo-dispositivos.png) |
| En vivo · Mapa 2D | [05-en-vivo-mapa-2d.png](05-en-vivo-mapa-2d.png) |
| En vivo · Servicios | [06-en-vivo-servicios.png](06-en-vivo-servicios.png) |
| Investigar · Workspace | [07-investigar-workspace.png](07-investigar-workspace.png) |
| Investigar · Timeline y evidencia | [08-investigar-timeline-evidencia.png](08-investigar-timeline-evidencia.png) |
| Investigar · Sesiones | [09-investigar-sesiones.png](09-investigar-sesiones.png) |
| Exposición | [10-exposicion.png](10-exposicion.png) |
| Ajustes · DNS externo | [11-ajustes-dns.png](11-ajustes-dns.png) |
| Diagnóstico | [12-diagnostico.png](12-diagnostico.png) |
| Comandos · Liquid Glass | [13-comandos-liquid-glass.png](13-comandos-liquid-glass.png) |

## Criterios para la implementación

Usa las imágenes como referencia de composición, jerarquía y materiales. Comprueba textos, valores, estados y acciones con las capturas originales y el producto antes de implementarlos. La tipografía rasterizada y la geometría de un mapa generado no son fuentes de datos.

- Conserva el wordmark y la X originales. Usa Geist para la interfaz y Geist Mono para direcciones, puertos, fechas y medidas.
- Mantén las cinco áreas de navegación y permite reorganizar su contenido según la tarea.
- Da prioridad al objeto investigado. Sitúa sus acciones junto a él y lleva los detalles largos a un inspector.
- Usa mapas para destinos con coordenadas conocidas, agrupaciones para dispositivos, comparaciones para snapshots y timelines para evidencia temporal.
- Distingue vecinos conocidos de conexiones observadas. Una caché de vecinos no demuestra tráfico ni enlaces entre dispositivos.
- Conserva cero, ausencia de datos y atribuciones desconocidas. No añadas puntuaciones de riesgo, veredictos de malware ni geolocalizaciones inventadas.
- Evita repetir explicaciones en cada fila. Conserva una explicación breve o un detalle consultable cuando afecte a la interpretación.
- Usa Liquid Glass en la paleta de comandos, los controles flotantes de mapa y los selectores temporales. Mantén opacas las tablas, los campos y la evidencia principal.
- Haz que el cristal mantenga el contraste del texto con cualquier contenido situado detrás. Ofrece una alternativa opaca con transparencia reducida.
- Mantén etiquetas visibles en los campos, foco de teclado, selección reconocible y estados que no dependan solo del color.

## Paleta de referencia

| Uso | Color |
| --- | --- |
| Fondo | `#101010` |
| Navegación | `#151413` |
| Panel | `#1d1a18` |
| Separador | `#3d3a39` |
| Texto secundario | `#b8b3b0` |
| Texto principal | `#eeeeee` |
| Captura y ubicación local | `#a0ca92` |
| Evidencia seleccionada o cambio | `#ee6018` |

El efecto de cristal usa los tonos del entorno. Evita introducir azul, cian o púrpura como nueva identidad.

Estas referencias no modifican la aplicación. La revisión cubre la composición de las imágenes; el comportamiento y la accesibilidad se comprueban al implementar la interfaz.
