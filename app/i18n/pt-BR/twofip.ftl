# 2fip — Português (Brasil)
# Tradução completa. O português de Portugal (i18n/pt) só redefine as chaves
# que mudam; o resto vem deste arquivo.

## Barra de menus
menu-file = Arquivo
menu-edit = Editar
menu-view = Exibir
menu-help = Ajuda
help = Ajuda
new-tab = Nova aba
new-folder = Nova pasta
close-tab = Fechar aba
quit = Sair
cut = Recortar
copy = Copiar
paste = Colar
select-all = Selecionar tudo
rename-ellipsis = Renomear…
delete = Excluir
list-view = Exibição em lista
grid-view = Exibição em grade
favorites = Favoritos
settings = Configurações

## Menu de contexto
open = Abrir
open-with-ellipsis = Abrir com…
view = Visualizar
edit = Editar
calculate-size = Calcular tamanho
compress-ellipsis = Compactar…
show-details = Mostrar detalhes

## Colunas da lista
column-name = Nome
column-ext = Ext.
column-size = Tamanho
column-modified = Modificado

## Operações em andamento (painel de progresso)
op-copying = Copiando
op-moving = Movendo
op-deleting = Excluindo
op-compressing = Compactando
op-copy-failed = Falha ao copiar
op-move-failed = Falha ao mover
op-delete-failed = Falha ao excluir
op-compress-failed = Falha ao compactar
op-progress = { $operation } { $percent }%
op-files-done = { $done } de { $total } arquivos
op-bytes-done = { $done } de { $total }
op-preparing = Preparando…
close = Fechar
cancel = Cancelar

## Painéis
path-not-found = Não encontrado: { $path }
status-items = { $count ->
    [one] { $count } item
   *[other] { $count } itens
}
status-items-filtered = { $count ->
    [one] { $count } item corresponde ao filtro
   *[other] { $count } itens correspondem ao filtro
}
status-selected = { $selected } de { $total } selecionados ({ $size })
tooltip-back = Voltar (Alt+←)
tooltip-forward = Avançar (Alt+→)
tooltip-up = Pasta acima (Backspace)
tooltip-drive = Unidade
tooltip-new-tab = Nova aba (Ctrl+T)
tooltip-refresh = Atualizar os dois painéis (Ctrl+R)
tooltip-edit-path = Editar caminho (Ctrl+L)
tooltip-cancel-esc = Cancelar (Esc)
no-tab-open = Nenhuma aba aberta
filter-placeholder = Filtrar (ex.: relatorio ou *.txt)
path-placeholder = Digite o caminho de uma pasta

## Configurações
settings-general = Geral
settings-show-hidden = Mostrar arquivos ocultos
settings-show-hidden-description = Arquivos e pastas cujo nome começa com um ponto
settings-separate-ext = Mostrar a extensão em uma coluna própria
settings-separate-ext-description = Exibição em lista: colunas Nome e Ext. separadas, como no Total Commander
settings-thumbnails = Mostrar miniaturas
settings-thumbnails-description = Pré-visualizações de imagens, PDFs, vídeos e fontes
settings-language = Idioma
language-system = Padrão do sistema
settings-theme = Tema
settings-color-theme = Tema de cores
settings-color-theme-description = Afeta apenas o 2fip; "Sistema" segue a aparência do COSMIC
color-theme-system = Sistema
color-theme-light = Claro
color-theme-dark = Escuro
settings-icon-style = Estilo dos ícones
settings-icon-style-description = "Colorido" é igual ao app COSMIC Files
icon-style-colorful = Colorido
icon-style-monochrome = Monocromático
icon-style-vivid = Vívido
icon-style-classic = Estilo Windows
icon-style-soft = Estilo macOS
settings-font-size = Tamanho dos nomes de arquivo
settings-font-size-description = Um texto menor mostra mais arquivos na tela
font-size-default = Padrão ({ $px } px)
font-size-small = Pequeno ({ $px } px)
font-size-smaller = Menor ({ $px } px)
font-size-tiny = Minúsculo ({ $px } px)
settings-corners = Cantos arredondados
settings-corners-description = Quão arredondados são a janela, os botões e os painéis
corners-square = Retos
corners-small = Pouco arredondados
corners-medium = Arredondados
corners-large = Muito arredondados (COSMIC)

## Favoritos
favorites-saved = Pastas salvas
favorites-empty = Nenhum favorito salvo ainda.
favorites-move-up = Mover para cima (Ctrl+↑)
favorites-move-down = Mover para baixo (Ctrl+↓)
favorites-add-current = Adicionar a pasta atual
favorites-add = Adicionar

## Diálogo "O arquivo já existe"
conflict-title = O arquivo já existe
conflict-body = "{ $existing }" já existe no destino. Substituir por "{ $new }"?
skip-all = Pular todos
replace-all = Substituir todos
replace = Substituir
skip = Pular

## Diálogo Compactar
compress = Compactar
compress-one = Compactar "{ $name }" em um arquivo ZIP.
compress-many = Compactar { $count } itens em um arquivo ZIP.
archive-name = Nome do arquivo compactado
default-archive-name = Arquivo

## Diálogo Nova pasta
new-folder-default-name = Nova pasta
folder-name = Nome da pasta
create = Criar

## Diálogo Renomear
rename = Renomear
renaming = Renomeando "{ $name }"
new-name = Novo nome

## Diálogo Excluir
delete-one = Mover "{ $name }" para a lixeira?
delete-many = Mover { $count } itens para a lixeira?
delete-permanently = Excluir permanentemente
delete-permanently-one = Excluir "{ $name }" permanentemente? Isso não pode ser desfeito.
delete-permanently-many = Excluir { $count } itens permanentemente? Isso não pode ser desfeito.

## Barra de teclas de função (o app adiciona "F2" etc.)
fkey-rename = Renomear
fkey-view = Visualizar
fkey-edit = Editar
fkey-copy = Copiar
fkey-move = Mover
fkey-mkdir = Nova pasta
fkey-delete = Excluir
fkey-terminal = Terminal
fkey-exit = Sair

## Diálogo Abrir com
open-with = Abrir com
open-with-one = Escolha um aplicativo para abrir "{ $name }".
open-with-many = Escolha um aplicativo para abrir { $count } itens.
loading-apps = Carregando aplicativos…
no-apps = Nenhum aplicativo encontrado.
recommended-apps = Aplicativos recomendados
other-apps = Outros aplicativos
default-app = Padrão

## Diálogo Detalhes
details-items-title = { $count } itens
details-calculating = Calculando…
details-error = Erro
details-contents = { $files } arquivos e { $folders } pastas dentro
details-size = { $size } ({ $bytes } bytes)
details-items = Itens
details-location = Local
details-total-size = Tamanho total
details-type = Tipo
details-name = Nome
details-link-target = Destino do link
details-size-label = Tamanho
details-modified = Modificado
details-accessed = Último acesso
details-created = Criado
details-permissions = Permissões
details-owner = Proprietário
details-group = Grupo
details-unknown = Desconhecido
kind-folder = Pasta
kind-file = Arquivo
kind-symlink = Link simbólico

## Diálogo Localizar arquivos
find = Localizar arquivos
tooltip-find = Localizar arquivos (Ctrl+F)
find-in = Em { $path } e suas subpastas
find-placeholder = Nome ou padrão (ex.: relatorio ou *.pdf)
find-search = Pesquisar
find-stop = Parar
find-hint = Digite parte de um nome e pressione Enter.
find-searching = Pesquisando… { $count } encontrados
find-results = { $count ->
    [one] 1 resultado
   *[other] { $count } resultados
}
find-no-results = Nada encontrado.
find-truncated = Mostrando os primeiros { $count } resultados; tente um nome mais específico.
find-go-to = Ir para o arquivo

## Janela Sobre
about-2fip = Sobre o 2fip
about-description = Um gerenciador de arquivos de dois painéis, feito para o teclado e inspirado no Total Commander.
about-version = Versão:
about-repository = Repositório:
about-license = Licença:

## Diálogo Conectar a um servidor
connect-to-server = Conectar a um servidor…
disconnect = Desconectar
connect-title = Conectar a um servidor
connect = Conectar
connect-host = Servidor (ex.: arquivos.exemplo.com)
connect-port = Porta
connect-user = Usuário
connect-password = Senha
connect-hint-sftp = Primeiro são testados o seu agente SSH e as chaves em ~/.ssh, então a senha é opcional.
connect-hint-ftp = Deixe o usuário vazio para entrar de forma anônima. O FTP envia a senha sem criptografia: prefira FTPS ou SFTP.
connect-hint-ftps = FTP criptografado com TLS. O servidor precisa de um certificado válido.
connect-connecting = Conectando…
connect-error-host = Digite o nome ou o endereço do servidor.
connect-error-port = A porta deve ser um número.
connect-error-login = Falha no login: verifique o usuário e a senha.
connect-unknown-host-title = Servidor desconhecido
connect-unknown-host-body = O 2fip nunca se conectou a { $host }. Verifique se esta impressão digital da chave corresponde à do servidor e confie nela para conectar. Ela será salva em ~/.ssh/known_hosts.
connect-trust = Confiar e conectar
connect-key-changed-title = A chave do servidor mudou
connect-key-changed-body = A chave de { $host } não corresponde à salva em ~/.ssh/known_hosts. Alguém pode estar interceptando a conexão, então o 2fip não vai conectar. Se o servidor foi reinstalado, remova a linha antiga dele de ~/.ssh/known_hosts.

## Painel Conexões
connections = Conexões
tooltip-connections = Conexões
connection-add = Adicionar conexão
connection-edit = Editar conexão
connection-save-only = Salvar
connection-save = Salvar conexão
connection-open = Abrir
connection-reconnect = Reconectar
connection-status-busy = Conectando…
connection-status-connected = Conectado
connection-status-lost = Conexão perdida
connection-status-closed = Desconectado
connections-saved = Salvas
connections-open-unsaved = Abertas, não salvas
connections-empty = Nenhuma conexão salva ainda.
connections-hint = As senhas que você escolher lembrar ficam criptografadas no chaveiro do sistema (GNOME Keyring, KWallet), nunca nos arquivos do 2fip.
connect-name = Nome (opcional)
connect-password-stored = Salva no chaveiro (digite para alterar)
connect-save = Salvar em Conexões
connect-remember-password = Lembrar a senha no chaveiro do sistema
