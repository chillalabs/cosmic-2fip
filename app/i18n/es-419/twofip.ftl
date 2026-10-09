# 2fip — Español (Latinoamérica)
# Traducción completa. El español de España (i18n/es) solo redefine las
# claves que cambian; el resto se toma de este archivo.

## Barra de menú
menu-file = Archivo
menu-edit = Editar
menu-view = Ver
menu-help = Ayuda
help = Ayuda
new-tab = Nueva pestaña
new-folder = Nueva carpeta
close-tab = Cerrar pestaña
quit = Salir
cut = Cortar
copy = Copiar
paste = Pegar
select-all = Seleccionar todo
rename-ellipsis = Cambiar nombre…
delete = Eliminar
list-view = Vista de lista
grid-view = Vista de cuadrícula
favorites = Favoritos
settings = Configuración

## Menú contextual
open = Abrir
open-with-ellipsis = Abrir con…
view = Ver
edit = Editar
calculate-size = Calcular tamaño
compress-ellipsis = Comprimir…
show-details = Ver detalles

## Columnas de la lista
column-name = Nombre
column-ext = Ext
column-size = Tamaño
column-modified = Modificado

## Operaciones en curso (panel de progreso)
op-copying = Copiando
op-moving = Moviendo
op-deleting = Eliminando
op-compressing = Comprimiendo
op-copy-failed = Error al copiar
op-move-failed = Error al mover
op-delete-failed = Error al eliminar
op-compress-failed = Error al comprimir
op-progress = { $operation } { $percent }%
op-files-done = { $done } de { $total } archivos
op-bytes-done = { $done } de { $total }
op-preparing = Preparando…
close = Cerrar
cancel = Cancelar

## Paneles
path-not-found = No se encontró: { $path }
status-items = { $count ->
    [one] 1 elemento
   *[other] { $count } elementos
}
status-items-filtered = { $count ->
    [one] 1 elemento coincide con el filtro
   *[other] { $count } elementos coinciden con el filtro
}
status-selected = { $selected } de { $total } seleccionados ({ $size })
tooltip-back = Atrás (Alt+←)
tooltip-forward = Adelante (Alt+→)
tooltip-up = Subir una carpeta (Retroceso)
tooltip-drive = Unidad
tooltip-new-tab = Nueva pestaña (Ctrl+T)
tooltip-refresh = Actualizar ambos paneles (Ctrl+R)
tooltip-edit-path = Editar ruta (Ctrl+L)
tooltip-cancel-esc = Cancelar (Esc)
no-tab-open = No hay pestañas abiertas
filter-placeholder = Filtrar (p. ej., informe o *.txt)
path-placeholder = Escribe la ruta de una carpeta

## Configuración
settings-general = General
settings-show-hidden = Mostrar archivos ocultos
settings-show-hidden-description = Archivos y carpetas cuyo nombre empieza con un punto
settings-separate-ext = Mostrar la extensión en su propia columna
settings-separate-ext-description = Vista de lista: columnas Nombre y Ext separadas, como en Total Commander
settings-thumbnails = Mostrar miniaturas
settings-thumbnails-description = Vistas previas de imágenes, PDF, videos y fuentes
settings-language = Idioma
language-system = Predeterminado del sistema
settings-theme = Tema
settings-color-theme = Tema de colores
settings-color-theme-description = Solo afecta a 2fip; Sistema sigue la apariencia de COSMIC
color-theme-system = Sistema
color-theme-light = Claro
color-theme-dark = Oscuro
settings-icon-style = Estilo de íconos
settings-icon-style-description = El estilo colorido es igual al de la aplicación COSMIC Files
icon-style-colorful = Colorido
icon-style-monochrome = Monocromático
icon-style-vivid = Vívido
icon-style-classic = Estilo Windows
icon-style-soft = Estilo macOS
settings-font-size = Tamaño de los nombres de archivo
settings-font-size-description = Un texto más pequeño muestra más archivos en pantalla
font-size-default = Predeterminado ({ $px } px)
font-size-small = Pequeño ({ $px } px)
font-size-smaller = Más pequeño ({ $px } px)
font-size-tiny = Diminuto ({ $px } px)
settings-corners = Esquinas redondeadas
settings-corners-description = Cuán redondeados son la ventana, los botones y los paneles
corners-square = Rectas
corners-small = Poco redondeadas
corners-medium = Redondeadas
corners-large = Muy redondeadas (COSMIC)

## Favoritos
favorites-saved = Carpetas guardadas
favorites-empty = Aún no hay favoritos guardados.
favorites-move-up = Subir (Ctrl+↑)
favorites-move-down = Bajar (Ctrl+↓)
favorites-add-current = Agregar la carpeta actual
favorites-add = Agregar

## Diálogo "El archivo ya existe"
conflict-title = El archivo ya existe
conflict-body = "{ $existing }" ya existe en el destino. ¿Quieres reemplazarlo por "{ $new }"?
skip-all = Omitir todos
replace-all = Reemplazar todos
replace = Reemplazar
skip = Omitir

## Diálogo Comprimir
compress = Comprimir
compress-one = Comprimir "{ $name }" en un archivo zip.
compress-many = Comprimir { $count } elementos en un archivo zip.
archive-name = Nombre del archivo comprimido
default-archive-name = Comprimido

## Diálogo Nueva carpeta
new-folder-default-name = Nueva carpeta
folder-name = Nombre de la carpeta
create = Crear

## Diálogo Cambiar nombre
rename = Cambiar nombre
renaming = Cambiando el nombre de "{ $name }"
new-name = Nombre nuevo

## Diálogo Eliminar
delete-one = ¿Mover "{ $name }" a la papelera?
delete-many = ¿Mover { $count } elementos a la papelera?
delete-permanently = Eliminar definitivamente
delete-permanently-one = ¿Eliminar "{ $name }" definitivamente? No se puede deshacer.
delete-permanently-many = ¿Eliminar { $count } elementos definitivamente? No se puede deshacer.

## Barra de teclas de función (la app agrega el prefijo "F2", etc.)
fkey-rename = Renombrar
fkey-view = Ver
fkey-edit = Editar
fkey-copy = Copiar
fkey-move = Mover
fkey-mkdir = Crear carpeta
fkey-delete = Eliminar
fkey-terminal = Terminal
fkey-exit = Salir

## Diálogo Abrir con
open-with = Abrir con
open-with-one = Elige una aplicación para abrir "{ $name }".
open-with-many = Elige una aplicación para abrir { $count } elementos.
loading-apps = Cargando aplicaciones…
no-apps = No se encontraron aplicaciones.
recommended-apps = Aplicaciones recomendadas
other-apps = Otras aplicaciones
default-app = Predeterminada

## Diálogo Detalles
details-items-title = { $count } elementos
details-calculating = Calculando…
details-error = Error
details-contents = { $files } archivos y { $folders } carpetas en su interior
details-size = { $size } ({ $bytes } bytes)
details-items = Elementos
details-location = Ubicación
details-total-size = Tamaño total
details-type = Tipo
details-name = Nombre
details-link-target = Destino del enlace
details-size-label = Tamaño
details-modified = Modificado
details-accessed = Último acceso
details-created = Creado
details-permissions = Permisos
details-owner = Propietario
details-group = Grupo
details-unknown = Desconocido
kind-folder = Carpeta
kind-file = Archivo
kind-symlink = Enlace simbólico

## Diálogo Buscar archivos
find = Buscar archivos
tooltip-find = Buscar archivos (Ctrl+F)
find-in = En { $path } y sus subcarpetas
find-placeholder = Nombre o patrón (p. ej. informe o *.pdf)
find-search = Buscar
find-stop = Detener
find-hint = Escribe parte de un nombre y presiona Enter.
find-searching = Buscando… { $count } encontrados
find-results = { $count ->
    [one] 1 resultado
   *[other] { $count } resultados
}
find-no-results = No se encontró nada.
find-truncated = Se muestran los primeros { $count } resultados; prueba un nombre más específico.
find-go-to = Ir al archivo

## Ventana Acerca de
about-2fip = Acerca de 2fip
about-description = Un administrador de archivos de dos paneles, pensado para el teclado e inspirado en Total Commander.
about-version = Versión:
about-repository = Repositorio:
about-license = Licencia:

## Diálogo Conectar a un servidor
connect-to-server = Conectar a un servidor…
disconnect = Desconectar
connect-title = Conectar a un servidor
connect = Conectar
connect-host = Servidor (p. ej. archivos.ejemplo.com)
connect-port = Puerto
connect-user = Usuario
connect-password = Contraseña
connect-hint-sftp = Primero se prueban tu agente SSH y las claves de ~/.ssh, así que la contraseña es opcional.
connect-hint-ftp = Deja el usuario vacío para entrar de forma anónima. FTP envía la contraseña sin cifrar: mejor usa FTPS o SFTP.
connect-hint-ftps = FTP cifrado con TLS. El servidor necesita un certificado válido.
connect-connecting = Conectando…
connect-error-host = Escribe el nombre o la dirección del servidor.
connect-error-port = El puerto debe ser un número.
connect-error-login = No se pudo iniciar sesión: revisa el usuario y la contraseña.
connect-unknown-host-title = Servidor desconocido
connect-unknown-host-body = 2fip nunca se conectó a { $host }. Comprueba que esta huella de la clave coincida con la del servidor y confía en ella para conectarte. Se guardará en ~/.ssh/known_hosts.
connect-trust = Confiar y conectar
connect-key-changed-title = La clave del servidor cambió
connect-key-changed-body = La clave de { $host } no coincide con la guardada en ~/.ssh/known_hosts. Alguien podría estar interceptando la conexión, así que 2fip no se conectará. Si reinstalaron el servidor, borra su línea antigua de ~/.ssh/known_hosts.

## Panel Conexiones
connections = Conexiones
tooltip-connections = Conexiones
connection-add = Agregar conexión
connection-edit = Editar conexión
connection-save-only = Guardar
connection-save = Guardar conexión
connection-open = Abrir
connection-reconnect = Reconectar
connection-status-busy = Conectando…
connection-status-connected = Conectado
connection-status-lost = Conexión perdida
connection-status-closed = Sin conectar
connections-saved = Guardadas
connections-open-unsaved = Abiertas, sin guardar
connections-empty = Todavía no hay conexiones guardadas.
connections-hint = Las contraseñas que elijas recordar se guardan cifradas en el llavero del sistema (GNOME Keyring, KWallet), nunca en los archivos de 2fip.
connect-name = Nombre (opcional)
connect-password-stored = Guardada en el llavero (escribe para cambiarla)
connect-save = Guardar en Conexiones
connect-remember-password = Recordar la contraseña en el llavero del sistema
