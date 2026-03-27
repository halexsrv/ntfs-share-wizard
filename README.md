# ntfs-share-wizard

## Visao geral do projeto

`ntfs-share-wizard` e um executavel TUI escrito em Rust para orientar a preparacao de uma particao NTFS compartilhada entre Windows e Linux, com foco no uso da mesma biblioteca Steam nos dois sistemas.

O projeto foi estruturado desde o inicio com separacao real por sistema operacional:

- codigo de Windows apenas em `src/windows/*`
- codigo de Linux apenas em `src/linux/*`
- deteccao de sistema em runtime em `src/os/*`
- camada de TUI em `src/tui/*`

## Objetivo do executavel

O objetivo do executavel e reduzir erros comuns ao preparar um disco NTFS para uso compartilhado entre Windows e Linux, especialmente para a pasta de biblioteca Steam em:

`/media/gamedisk/SteamLibrary`

Hoje o wizard cobre:

- deteccao do sistema operacional em runtime
- fluxo Windows para desabilitar Fast Startup
- fluxo Linux para detectar distro
- validacao de `ntfs-3g`
- deteccao de particoes NTFS com `lsblk --json`
- validacao de mountpoint e pasta `SteamLibrary`
- geracao da linha de `fstab`
- backup e escrita segura em `/etc/fstab`
- aplicacao de `mount -a`
- validacao de montagem, escrita e existencia da `SteamLibrary`
- orientacao final de compartilhamento entre Windows e Linux

## Fluxo no Windows

No Windows, o wizard mostra um fluxo dedicado para desabilitar Fast Startup.

Etapas atuais:

1. explicacao do que e Fast Startup
2. motivo pelo qual ele deve ser desabilitado para compartilhamento seguro de NTFS
3. confirmacao antes da execucao
4. execucao de `powercfg /h off`
5. captura de `stdout`, `stderr` e codigo de saida
6. recomendacao final para desligamento completo com:

```powershell
shutdown /s /t 0
```

## Fluxo no Linux

No Linux, o wizard segue um fluxo guiado para preparar a particao NTFS e o caminho padrao da biblioteca Steam.

Etapas atuais:

1. deteccao da distribuicao Linux
2. validacao de presenca de `ntfs-3g`
3. exibicao de plano de instalacao por distribuicao
4. execucao assistida de instalacao quando suportado
5. deteccao de particoes NTFS reais via `lsblk --json`
6. selecao de particao na TUI
7. validacao de `/media/gamedisk`
8. validacao de `/media/gamedisk/SteamLibrary`
9. criacao opcional de diretorios quando seguro
10. revisao da linha de `fstab`
11. backup e escrita segura em `/etc/fstab`
12. execucao de `mount -a`
13. validacao de montagem, leitura e escrita
14. criacao opcional da pasta `SteamLibrary`
15. tela final com orientacoes de compartilhamento entre Windows e Linux

## Sistemas operacionais suportados

- Windows
- Linux

## Distribuicoes Linux suportadas atualmente

- Ubuntu
- SteamOS
- Bazzite
- Arch Linux
- Fedora

Quando a distribuicao nao pode ser identificada, o projeto trata o caso como `Unknown` e usa apenas orientacao guiada, sem assumir comandos automatizados.

## Tecnologias e bibliotecas usadas no projeto

- Rust
- Ratatui
- Crossterm
- Serde
- Serde JSON
- Anyhow

## Como compilar

Requisitos:

- Rust estavel
- Cargo

Comando:

```bash
cargo build
```

## Como rodar

```bash
cargo run
```

## Permissoes necessarias

Algumas etapas do projeto exigem privilegios adequados do sistema operacional.

### Windows

Para desabilitar Fast Startup com `powercfg /h off`, pode ser necessario executar o terminal com privilegios administrativos.

### Linux

As etapas abaixo podem exigir permissao elevada, dependendo do ambiente:

- instalar `ntfs-3g`
- criar diretorios em caminhos protegidos
- escrever em `/etc/fstab`
- executar `mount -a`

Sem permissao suficiente, o wizard tenta retornar mensagens amigaveis de erro em vez de encerrar com panic.

## Estrutura do projeto

```text
src/
  app.rs
  main.rs
  os/
    detect.rs
    mod.rs
  tui/
    mod.rs
  windows/
    mod.rs
    system.rs
    wizard.rs
  linux/
    mod.rs
    system.rs
    fstab.rs
    mount.rs
    wizard.rs
```

### Resumo da responsabilidade por modulo

- `src/main.rs`: inicializacao do binario
- `src/app.rs`: estado global do app e navegacao principal
- `src/os/*`: deteccao de sistema operacional
- `src/tui/mod.rs`: renderizacao e loop de eventos da TUI
- `src/windows/system.rs`: operacoes especificas de Windows
- `src/windows/wizard.rs`: telas e fluxo Windows
- `src/linux/system.rs`: deteccao de distro, `ntfs-3g`, particoes, paths e geracao de `fstab`
- `src/linux/fstab.rs`: backup e escrita segura em `/etc/fstab`
- `src/linux/mount.rs`: aplicacao da montagem e validacoes pos-configuracao
- `src/linux/wizard.rs`: telas e fluxo Linux

## Padrao adotado no projeto para o fstab

O projeto adota explicitamente o seguinte formato para a entrada do `/etc/fstab`:

```fstab
UUID=<uuid> /media/gamedisk ntfs-3g uid=1000,gid=1000,rw,noatime,user,exec,umask=022,nofail 0 0
```

Esse padrao e gerado pelo fluxo Linux usando:

- filesystem exatamente `ntfs-3g`
- mountpoint exatamente `/media/gamedisk`
- opcoes exatamente:
  - `uid=1000`
  - `gid=1000`
  - `rw`
  - `noatime`
  - `user`
  - `exec`
  - `umask=022`
  - `nofail`

## Observacoes sobre `/media/gamedisk`

- este e o mountpoint padrao adotado pelo projeto
- o wizard valida explicitamente se esse caminho existe
- se o caminho nao existir e for seguro criar, o wizard oferece criacao
- o fluxo nao usa symlink para esse mountpoint

## Observacoes sobre `/media/gamedisk/SteamLibrary`

- este e o caminho padrao adotado para a biblioteca Steam compartilhada
- o wizard valida explicitamente se a pasta existe
- se a montagem estiver funcional e a pasta nao existir, o wizard pode oferecer criacao
- o fluxo foi desenhado para que Windows e Linux apontem para a mesma pasta real
- o projeto evita symlink nesse caminho

## Observacoes sobre Fast Startup no Windows

Fast Startup pode deixar a particao NTFS em um estado hibrido ou inseguro para montagem no Linux.

Quando o fluxo Linux detecta sinais de volume inseguro, modo somente leitura ou mensagens compativeis com hibernacao/estado inconsistente, o wizard reforca a necessidade de corrigir isso no Windows.

Fluxo recomendado:

```powershell
powercfg /h off
shutdown /s /t 0
```

## Observacoes sobre `ntfs-3g`

- o fluxo Linux depende de `ntfs-3g` para a entrada de `fstab` adotada pelo projeto
- o wizard verifica se `ntfs-3g` esta disponivel no `PATH`
- se nao estiver, o app mostra um plano por distribuicao
- em Ubuntu, SteamOS, Arch Linux e Fedora existe fluxo assistido de execucao
- em Bazzite o fluxo e propositalmente conservador, sem inventar instalacao arriscada

## Limitacoes conhecidas

- o projeto e um wizard TUI inicial e ainda nao cobre todos os cenarios de disco, Steam e permissao possiveis
- o suporte Linux atual e focado nas distribuicoes listadas neste README
- o caso Bazzite e tratado com cautela e nao tenta instalacao automatica arriscada
- o padrao de `fstab` usa `uid=1000` e `gid=1000`, o que pode nao corresponder a todos os usuarios ou ambientes
- o wizard trabalha com o mountpoint padrao `/media/gamedisk` e a pasta padrao `/media/gamedisk/SteamLibrary`
- o projeto depende de comandos externos do sistema, como `lsblk`, `mount` e gerenciadores de pacote
- o suporte a Windows hoje cobre especificamente o fluxo de Fast Startup

## Consistencia com o estado atual do projeto

Este README descreve o comportamento implementado atualmente no codigo, incluindo:

- suporte a Windows e Linux
- suporte atual a Ubuntu, SteamOS, Bazzite, Arch Linux e Fedora
- uso de Rust, Ratatui, Crossterm, Serde, Serde JSON e Anyhow
- padrao exato de `fstab` adotado pelo projeto
