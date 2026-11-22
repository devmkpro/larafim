// Sem janela de console no executável final.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Execução elevada (UAC) só para gravar o arquivo hosts.
    if let Some(code) = larafim_server::hosts::run_helper_if_requested() {
        std::process::exit(code);
    }
    larafim_app_lib::run()
}
