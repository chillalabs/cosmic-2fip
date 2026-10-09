# 2fip — Português (Portugal)
# Só as chaves que diferem do português do Brasil (i18n/pt-BR); tudo o resto
# vem desse ficheiro.

menu-file = Ficheiro
menu-view = Ver
new-tab = Novo separador
close-tab = Fechar separador
rename-ellipsis = Mudar o nome…
delete = Eliminar
list-view = Vista de lista
grid-view = Vista de grelha
settings = Definições

view = Ver
compress-ellipsis = Comprimir…

op-copying = A copiar
op-moving = A mover
op-deleting = A eliminar
op-compressing = A comprimir
op-delete-failed = Falha ao eliminar
op-compress-failed = Falha ao comprimir
op-files-done = { $done } de { $total } ficheiros
op-preparing = A preparar…

tooltip-new-tab = Novo separador (Ctrl+T)
tooltip-refresh = Atualizar os dois painéis (Ctrl+R)
no-tab-open = Nenhum separador aberto
filter-placeholder = Filtrar (p. ex. relatorio ou *.txt)
path-placeholder = Escreva o caminho de uma pasta

settings-show-hidden = Mostrar ficheiros ocultos
settings-show-hidden-description = Ficheiros e pastas cujo nome começa por um ponto
settings-separate-ext-description = Vista de lista: colunas Nome e Ext. separadas, como no Total Commander
settings-icon-style-description = «Colorido» é igual à aplicação COSMIC Files
settings-font-size = Tamanho dos nomes de ficheiro
settings-color-theme-description = Afeta apenas o 2fip; «Sistema» segue a aparência do COSMIC
settings-font-size-description = Um texto mais pequeno mostra mais ficheiros no ecrã
font-size-smaller = Mais pequeno ({ $px } px)

favorites-saved = Pastas guardadas
favorites-empty = Ainda não há favoritos guardados.

conflict-title = O ficheiro já existe
conflict-body = «{ $existing }» já existe no destino. Substituir por «{ $new }»?
skip-all = Ignorar todos
skip = Ignorar

compress = Comprimir
compress-one = Comprimir «{ $name }» num ficheiro ZIP.
compress-many = Comprimir { $count } itens num ficheiro ZIP.
archive-name = Nome do ficheiro comprimido

rename = Mudar o nome
renaming = A mudar o nome de «{ $name }»

delete-one = Mover «{ $name }» para o lixo?
delete-many = Mover { $count } itens para o lixo?
delete-permanently = Eliminar permanentemente
delete-permanently-one = Eliminar «{ $name }» permanentemente? Não é possível anular.
delete-permanently-many = Eliminar { $count } itens permanentemente? Não é possível anular.

fkey-delete = Eliminar

open-with-one = Escolha uma aplicação para abrir «{ $name }».
open-with-many = Escolha uma aplicação para abrir { $count } itens.
loading-apps = A carregar aplicações…
no-apps = Nenhuma aplicação encontrada.
recommended-apps = Aplicações recomendadas
other-apps = Outras aplicações

details-calculating = A calcular…
details-contents = { $files } ficheiros e { $folders } pastas no interior
kind-file = Ficheiro

## Diálogo Procurar ficheiros
find = Procurar ficheiros
tooltip-find = Procurar ficheiros (Ctrl+F)
find-hint = Escreva parte de um nome e prima Enter.
find-searching = A pesquisar… { $count } encontrados
find-truncated = A mostrar os primeiros { $count } resultados; experimente um nome mais específico.
find-go-to = Ir para o ficheiro

## Janela Sobre
about-description = Um gestor de ficheiros de dois painéis, pensado para o teclado e inspirado no Total Commander.

## Diálogo Ligar a um servidor
connect-to-server = Ligar a um servidor…
disconnect = Desligar
connect-title = Ligar a um servidor
connect = Ligar
connect-user = Nome de utilizador
connect-password = Palavra-passe
connect-hint-sftp = Primeiro são experimentados o seu agente SSH e as chaves em ~/.ssh, por isso a palavra-passe é opcional.
connect-hint-ftp = Deixe o utilizador vazio para entrar de forma anónima. O FTP envia a palavra-passe sem cifra: prefira FTPS ou SFTP.
connect-connecting = A ligar…
connect-error-login = Falha ao iniciar sessão: verifique o utilizador e a palavra-passe.
connect-unknown-host-body = O 2fip nunca se ligou a { $host }. Verifique se esta impressão digital da chave corresponde à do servidor e confie nela para se ligar. Será guardada em ~/.ssh/known_hosts.
connect-trust = Confiar e ligar
connect-key-changed-body = A chave de { $host } não corresponde à guardada em ~/.ssh/known_hosts. Alguém pode estar a intercetar a ligação, por isso o 2fip não se vai ligar. Se o servidor foi reinstalado, remova a linha antiga dele de ~/.ssh/known_hosts.

## Painel Ligações
connections = Ligações
tooltip-connections = Ligações
connection-add = Adicionar ligação
connection-edit = Editar ligação
connection-save-only = Guardar
connection-save = Guardar ligação
connection-open = Abrir
connection-reconnect = Voltar a ligar
connection-status-busy = A ligar…
connection-status-connected = Ligado
connection-status-lost = Ligação perdida
connection-status-closed = Desligado
connections-saved = Guardadas
connections-open-unsaved = Abertas, não guardadas
connections-empty = Ainda não há ligações guardadas.
connections-hint = As palavras-passe que escolher memorizar ficam cifradas no porta-chaves do sistema (GNOME Keyring, KWallet), nunca nos ficheiros do 2fip.
connect-name = Nome (opcional)
connect-password-stored = Guardada no porta-chaves (escreva para alterar)
connect-save = Guardar em Ligações
connect-remember-password = Memorizar a palavra-passe no porta-chaves do sistema
