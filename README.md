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

Por defecto, Pegatina **no guarda** lo que copias desde un gestor de contraseñas.
Si quieres que sí lo haga, activa **«Guardar contraseñas copiadas»**: es la primera opción de Configuración y también está en el menú del icono de Pegatina junto al reloj.
Se guardan cifradas con tu usuario de Windows y se ven ocultas (••••••••); con el icono del ojo las puedes mostrar.

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
