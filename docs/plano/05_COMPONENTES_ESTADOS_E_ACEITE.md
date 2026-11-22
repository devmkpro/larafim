# Larafim — Design system funcional e critérios de aceite

## Componentes reutilizáveis
AppShell; Sidebar; PageHeader; ProjectRow; StatusIndicator; RuntimeSelector; ServiceCard; ProgressOperation; ConfirmDialog; ErrorPanel; EmptyState; PathPicker; PortInput; CopyableUrl; EnvDiffPreview; LogViewer; TerminalPanel; ResourceMeter; Toast; StepWizard.

## Estados padronizados
idle, scanning, downloading, verifying, installing, starting, online, stopping, offline, updating, failed, permission_required, missing_dependency.
Cada estado operacional deve ter texto claro, ação possível e, quando cabível, logs técnicos expansíveis.

## Regras de UX
- Mostrar ações principais sem esconder funções críticas em menus.
- Evitar polling pesado e inicialização de serviços apenas para desenhar a tela.
- Carregar logs incrementalmente; terminal somente sob demanda.
- Preservar dados e arquivos de usuário; confirmação antes de editar `.env` e remover configuração.
- Mostrar URLs completas com protocolo e porta quando diferente da padrão.
- Diferenciar operação planejada, em andamento, concluída e falha.
- Usar controles acessíveis por teclado e mensagens de erro específicas.

## Contratos UI ↔ Rust (proposta, nomes não definidos no PLANO.md)
list_projects, scan_roots, import_project, update_project, set_project_online, list_php_versions, install_php_version, update_php_version, get_gateway_status, diagnose_ports, update_hosts, list_services, set_service_running, preview_env_changes, apply_env_changes, undo_env_changes, read_logs, get_resource_usage.
Cada comando retorna sucesso/erro estruturado; operações longas emitem eventos de progresso. IDs de projetos estáveis; não passar shell arbitrário sem validação.

## Critérios gerais
- O MVP termina na fase 3 e suporta versões PHP diferentes simultaneamente.
- Nenhum projeto encontrado inicia sem seleção/estado persistido.
- Falhas não devem causar perda de configurações ou arquivos.
- Componentes adicionais são entregues somente quando a fase técnica os suporta.
- Metas de memória do PLANO.md são objetivos a medir, não resultados garantidos.
