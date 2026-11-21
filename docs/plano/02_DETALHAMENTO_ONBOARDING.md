# Sentinel — Especificação funcional: primeiro acesso
Referência: PLANO.md. As telas abaixo são uma proposta de UX para executar as fases 0–3; o plano original não define um onboarding obrigatório.

## ONB-01 — Boas-vindas
**Objetivo:** apresentar instalação leve e configuração local.
**Componentes:** logo, descrição curta, progresso do assistente, botão Começar, botão Configurar depois (quando viável).
**Ações:** Começar → ONB-02; sair → fechar sem alterar projetos.
**Estados:** novo usuário, configuração parcial recuperável.
**Persistência:** somente etapa concluída.

## ONB-02 — Diagnóstico do sistema
**Objetivo:** identificar condições para rodar o gateway e PHP.
**Componentes:** resultados de sistema Windows, acesso a diretórios, disponibilidade das portas 80/443/3306 e permissões para `hosts`; detalhes técnicos recolhíveis.
**Botões:** Verificar novamente, Ver detalhes, Continuar.
**Regras:** conflito não deve encerrar o assistente; oferecer alternativa de porta e explicar que a URL incluirá a porta alternativa. Não encerrar processos de terceiros.
**Estados:** verificando, aprovado, aviso, bloqueio, falha inesperada.
**Backend:** diagnóstico somente leitura, identificação de processo dono da porta quando disponível.

## ONB-03 — Seleção de pastas raiz
**Objetivo:** definir uma ou mais pastas de descoberta.
**Campos:** lista de caminhos; opção de adicionar e remover.
**Botões:** Escolher pasta, Adicionar outra, Atualizar prévia, Voltar, Continuar.
**Comportamento:** examinar subpastas sob demanda, detectar `artisan`, `composer.json`, `index.php`, `package.json`, `.php-version`, `.nvmrc`. Mostrar contagem e prévia dos candidatos; não iniciar nenhum.
**Validações:** caminho existente, acessível e sem duplicidade; oferecer pular quando não houver projetos.
**Persistência:** `config.toml` em `%LOCALAPPDATA%\Sentinel`.

## ONB-04 — PHP inicial (opcional)
**Objetivo:** oferecer runtime compatível com projetos detectados.
**Componentes:** versões sugeridas, origem oficial, tamanho quando conhecido, status instalado/não instalado, progresso por etapa.
**Botões:** Instalar selecionadas, Pular, Tentar novamente, Cancelar download.
**Fluxo:** obter catálogo → baixar build NTS x64 → verificar SHA-256 → extrair → gerar `php.ini` → confirmar versão.
**Regras:** não instalar versões sem solicitação; manter arquivos parciais isolados; erros não devem corromper versões existentes.
**Estados:** sem internet, hash inválido, download interrompido, espaço insuficiente, sucesso.

## ONB-05 — Domínio e permissões
**Objetivo:** preparar URLs locais.
**Campos:** sufixo de domínio `.offline` (padrão), porta HTTP (80 por padrão).
**Componentes:** explicação do `hosts` e do pedido UAC; prévia da URL completa.
**Botões:** Testar porta, Aplicar, Tentar novamente, Continuar sem ativar.
**Regras:** somente pedir UAC quando realmente houver mudança no `hosts`; negar UAC não pode ser interpretado como sucesso. Se usar porta alternativa, mostrar `http://projeto.offline:PORTA`.
**Estados:** aguardando permissão, permitido, negado, conflito, sucesso.

## ONB-06 — Revisão e conclusão
**Componentes:** pastas selecionadas, runtimes instalados, domínio/porta, avisos pendentes.
**Botões:** Voltar e corrigir, Concluir, Abrir projetos.
**Resultado:** abrir dashboard ou projetos. Projetos detectados permanecem offline até serem explicitamente configurados/ativados.
**Persistência:** configuração consolidada, com retomada segura após falha.

## Critérios de aceite
- Assistente não altera repositórios nem inicia projetos descobertos.
- Reabrir o aplicativo após interrupção permite retomar.
- Erros de permissão, porta e download exibem causa e recuperação.
- Cada operação longa tem progresso, cancelamento seguro quando possível e resultado verificável.
