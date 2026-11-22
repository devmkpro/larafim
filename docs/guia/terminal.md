# Terminal

Com os atalhos do Larafim no `PATH`, `php`, `composer`, `node`, `npm` e `npx` usam, em **qualquer
terminal**, a versão do projeto da pasta atual.

```powershell
cd D:\www\loja      # projeto em PHP 8.3
php -v              # PHP 8.3.x
cd D:\www\blog      # projeto em PHP 8.5
php -v              # PHP 8.5.x
```

## Ligar

Em *Configurações › Terminal*:

1. **Instalar/reparar atalhos**: cria `php.exe`, `composer.exe`, `node.exe`… em
   `%LOCALAPPDATA%\Larafim\bin`. São cópias do `larafim-shim.exe`, um programa pequeno que só
   escolhe a versão e repassa os argumentos.
2. **Colocar no PATH**: adiciona essa pasta ao `PATH` **do seu usuário** (não precisa de
   administrador). Abra um terminal novo para valer.

Se houver outro `php.exe` no `PATH` do sistema, por exemplo do XAMPP, o Larafim avisa, porque ele
pode ser encontrado antes.

## Como a versão é escolhida

1. A variável `LARAFIM_PHP` (ex.: `$env:LARAFIM_PHP = "8.3"`), se definida.
2. O projeto do Larafim que contém a pasta atual (versão fixada ou a mais nova compatível).
3. `.php-version` ou `composer.json` da pasta ou de uma pasta acima.
4. A **versão padrão do terminal** (*Configurações › Terminal* ou o painel da bandeja).
5. A mais nova instalada.

Se nada servir, o atalho explica o motivo e sai com código 1. Ele nunca roda uma versão errada em
silêncio. **Testar uma pasta** (em *Configurações › Terminal*) mostra qual versão seria usada e por quê.

Para o Node, a lógica é a mesma, com `.nvmrc`, `.node-version` e `engines.node`.

## Terminal do projeto

O botão de terminal no painel do projeto abre o Windows Terminal (ou o `cmd`) já na pasta do projeto,
com os atalhos na frente do `PATH`, mesmo sem tê-los ligado no `PATH` do usuário.
