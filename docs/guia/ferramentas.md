# Ferramentas

## Visualizador de logs

Abre pelo botão de logs no painel do projeto ou pela bandeja (*Visualizador de logs*). Mostra os
arquivos `.log` de `storage\logs` (Laravel) ou `logs` do projeto, ao vivo.

- O `laravel.log` é separado em **entradas**, com nível (Error, Warning, Info, Debug), hora e
  mensagem. A mais nova fica no topo.
- **Filtros** por nível com contagem, **busca** no texto e **Agrupar** mensagens iguais (×N).
- Na entrada selecionada aparecem a exceção, o arquivo e a linha (relativos ao projeto), o
  **stack trace** com os trechos de `vendor/` recolhidos e o contexto em JSON.
- **Copiar** a entrada original ou **Markdown** pronto para uma issue.
- Setas ↑/↓ navegam pelas entradas; **Ao vivo** pausa e continua.

Logs de outros formatos aparecem uma linha por entrada.

## Xdebug

Em *Configurações › PHP*, por versão: **ligar Xdebug** baixa a DLL certa de
[xdebug.org](https://xdebug.org/download) (versão do PHP, NTS, x64, conferida pelo SHA-256 da página)
e ajusta o `php.ini` no modo `debug` com `start_with_request=trigger`. O Xdebug só entra em ação
quando o navegador ou a IDE pedem, para não deixar tudo lento. O Sentinel mostra o `launch.json`
pronto para o VS Code.

## Workers do Laravel

No painel de projetos Laravel: **fila** (`queue:work`), **agendador** (`schedule:work`) e **Reverb**
(`reverb:start`, quando instalado). Rodam como processos gerenciados, com log ao vivo, reinício e
volta automática quando o Sentinel abre.

## Novo projeto Laravel

**Novo Laravel** (na janela principal ou na bandeja) cria um projeto com `composer create-project`:
nome, pasta, versão do PHP, starter kit (nenhum, React, Vue, Livewire) e banco. A saída aparece ao
vivo; no fim, o projeto já entra na lista com o `.env` configurado.

O nome só aceita letras, números, `-` e `_`.

## Painel da bandeja

O **clique esquerdo** no ícone do Sentinel, perto do relógio, abre um painel com:

- **Abrir Sentinel** e **Novo projeto Laravel**;
- **E-mail** (Mailpit), **Visualizador de logs** e **Serviços**;
- os **projetos online** (clique abre no navegador) e **Desligar todos**;
- o **PHP do terminal**: a versão usada fora dos projetos (automática ou uma fixa);
- **Configurações** e **Sair**.

O painel some ao clicar fora ou com Esc. O **clique direito** abre o menu clássico, com as mesmas
ações principais.

## Iniciar com o Windows

*Configurações › Geral*: com essa opção, o Sentinel abre direto na bandeja ao entrar no Windows. Ele
usa a entrada do usuário em `HKCU\...\Run`, sem administrador.
