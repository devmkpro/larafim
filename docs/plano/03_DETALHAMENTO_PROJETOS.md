# Larafim — Especificação funcional: projetos
Base: PLANO.md, seções 4.1, 4.3 e roadmap.

## PROJ-01 — Lista de projetos
**Objetivo:** gerenciar projetos de todas as pastas raiz e avulsas.
**Componentes:** busca, filtro de tipo/status, tabela ou lista (nome, tipo, PHP/Node, URL, status, caminho), estado vazio e alertas.
**Botões:** Atualizar (reescaneia sob demanda), Adicionar projeto (pasta avulsa), Importar, Abrir detalhes, Abrir URL, Copiar URL.
**Toggle Online/Offline:** iniciar ou parar somente o projeto escolhido; mostrar iniciando/parando; persistir intenção em `projects.toml`; erro mantém estado real explícito.
**Estados:** nenhum projeto, descobertos ainda não importados, pasta ausente, runtime ausente, domínio ocupado, falha ao subir.
**Backend:** scan, listagem, iniciar/parar, status, salvar configuração.

## PROJ-02 — Importar projeto
**Etapa 1:** escolher pasta, confirmar leitura e detectar framework.
**Etapa 2:** revisar nome, tipo Laravel/PHP/Node/misto, PHP sugerido por `.php-version` ou `composer.json`, Node por `.nvmrc`/`engines.node`, docroot.
**Etapa 3:** editar domínio (prévia da URL completa), PHP minor, docroot, Node habilitado, comando e banco.
**Etapa 4:** validar duplicidade de domínio, caminho, versões disponíveis, conflitos e apresentar resumo.
**Botões:** Voltar, Continuar, Instalar runtime ausente, Importar, Cancelar.
**Resultado:** salvar em `%LOCALAPPDATA%\Larafim\projects.toml`, sem escrever arquivos no projeto. Não ativar automaticamente sem consentimento.
**Erros:** caminho inválido, conflito de domínio, arquivos de projeto inconsistentes.

## PROJ-03 — Detalhes do projeto
**Cabeçalho:** nome, framework, status, URL, pasta, PHP/Node.
**Ações:** Online/Offline, Abrir navegador, Copiar URL, Abrir pasta; VS Code/terminal disponíveis nas fases correspondentes.
**Abas:** Visão geral, Ambiente, Serviços (fase 6+), Logs/Terminal (fase 6+), Workers (fase 7+), Configuração.
**Visão geral:** status do gateway, dependências faltantes e avisos acionáveis.
**Estados:** iniciando, online, offline, erro, caminho removido, versão incompatível.

## PROJ-04 — Configuração do projeto
**Campos:** nome amigável, pasta, docroot, domínio, versão minor PHP, Node e script (quando suportados), serviços associados.
**Ações:** Salvar, Cancelar, Testar configuração, Salvar configuração no projeto, Remover da lista.
**Validação:** domínio único; docroot dentro da pasta e existente; runtime compatível; conflitos de porta.
**Persistência:** padrão somente `projects.toml`. Opção `.larafim.toml` cria arquivo local e registra exclusão em `.git/info/exclude`; opção explícita permite versionar.
**Remover da lista:** confirmar que arquivos do projeto NÃO serão excluídos. Se online, parar primeiro e atualizar hosts quando necessário.
**Erros:** falha de gravação, arquivo indisponível, UAC negado; manter configuração anterior em caso de falha.

## Fluxo de ativação
Clique Online → validar configuração e gateway → garantir resolução de domínio → iniciar serviços estritamente necessários → iniciar/provisionar pool PHP sob demanda → checar acesso → mostrar Online ou erro recuperável. Ao reiniciar o Larafim, respeitar somente projetos previamente marcados Online.

## Critérios de aceite
- Dois projetos podem usar PHP minor diferentes ao mesmo tempo.
- URL completa abre o projeto correto.
- Atualizar lista não altera configuração nem inicia processos.
- Desligar um projeto não derruba outro que compartilha o mesmo pool.
- Não há escrita automática em arquivos de origem.
