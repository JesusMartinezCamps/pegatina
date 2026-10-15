# Pegatina

Pegatina guarda todo lo que copias (texto, enlaces, imágenes y archivos) para que puedas volver a pegarlo cuando quieras.
Funciona en Windows 11, no necesita internet y nada de lo que copias sale de tu ordenador.

## Cómo instalarla

1. En esta página de GitHub, abre **Releases** (en la columna de la derecha) y entra en la última versión.
2. Descarga el instalador: es el archivo que termina en `-setup.exe`.
3. Ábrelo. Es normal que Windows muestre el aviso **«Windows protegió su PC»**: pasa porque la app todavía no está firmada con un certificado de pago, no porque tenga nada raro.
   Pulsa **«Más información»** y después **«Ejecutar de todas formas»**.
4. Sigue los pasos del instalador. Pegatina arranca sola cada vez que enciendes el ordenador (puedes quitarlo en Configuración).

## Cómo se usa

- **Copia como siempre** (Ctrl+C). Pegatina lo guarda sola, sin que tengas que hacer nada.
- **Ctrl+Alt+Espacio** abre el panel junto al ratón. Si prefieres otro atajo, cámbialo en Configuración.
- **Escribe** para buscar entre todo lo que has copiado.
- **Enter pega** el elemento elegido en la ventana donde estabas (doble clic también). Con las flechas ↑ ↓ eliges.
- El icono de **copiar** de cada elemento lo copia sin pegarlo. **Supr** lo borra y **Esc** cierra el panel.
- La **estrella** marca un favorito. Los favoritos no caducan nunca.
- Los **espacios** son como carpetas de colores para tener juntas las cosas de un tema (por ejemplo, «Trabajo»). Crea el primero con **+ Nuevo espacio** en la Biblioteca y mueve elementos desde el menú **···** de cada uno. Hasta que no creas uno, los espacios no aparecen.
- Los botones de abajo del panel abren la **Biblioteca** (todo lo guardado, con detalle, imágenes grandes y filtros) y la **Configuración**.

### Contraseñas

Pegatina no lee ni adivina qué es una contraseña: lo sabe porque el programa de origen (un gestor de contraseñas o el navegador al pulsar «copiar contraseña») avisa a Windows de que lo copiado es secreto. Esas copias, por defecto, **no se guardan**.
Si quieres que sí se guarden, activa **«Guardar contraseñas de navegadores y gestores»**: es la primera opción de Configuración y también está en el menú del icono de Pegatina junto al reloj.
Se guardan cifradas con tu usuario de Windows y se ven ocultas (••••••••); con el icono del ojo las puedes mostrar.

Dos límites que conviene saber:

- **Una contraseña copiada a mano** (de una nota, un documento, un chat o un correo) nadie la marca, así que Pegatina la guarda como texto normal y legible. Si pasa, bórrala del historial con **Supr**.
- **Lo que copias en una ventana de incógnito o privada** también llega marcado como secreto, y Pegatina lo trata igual que una contraseña: con la opción apagada no se guarda, y con ella encendida se guarda cifrado.

## Dónde se guardan tus datos

Todo queda en tu ordenador, en la carpeta `%APPDATA%\Pegatina` (los textos y la lista de lo copiado) y, dentro de ella, la carpeta `multimedia` (las imágenes; de los archivos copiados solo se apunta dónde están).
Puedes cambiar las dos carpetas en Configuración; lo que ya tienes guardado se mueve contigo.
No hay cuentas, ni nube, ni envío de datos: Pegatina nunca se conecta a internet.

El historial guarda los últimos 1000 elementos (puedes elegir 100, 500, 1000 o 5000). Los favoritos y lo que está en un espacio no caducan.

## Cómo desinstalarla

Abre **Configuración de Windows → Aplicaciones → Aplicaciones instaladas**, busca **Pegatina** y elige **Desinstalar**.
Tus datos no se borran solos: si ya no los quieres, elimina la carpeta `%APPDATA%\Pegatina` (o la que hayas elegido en Configuración).

## Licencia

MIT, ver [LICENSE](LICENSE).
