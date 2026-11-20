//! Garante que nenhum `php-cgi.exe` fica órfão. Cada processo de serviço
//! iniciado pelo Sentinel entra num Job Object do Windows com "matar tudo ao
//! fechar"; o Sentinel segura o único handle do job. Se o Sentinel sair,
//! travar ou for encerrado pelo Gerenciador de Tarefas, o handle fecha e o
//! Windows encerra esses processos.
//!
//! Só os serviços entram no job — navegador, terminal e editor abertos pelo
//! Sentinel continuam vivos depois que ele fecha.

#[cfg(windows)]
mod imp {
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    };

    struct Job(HANDLE);
    // SAFETY: o handle do job pode ser usado de qualquer thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    fn job() -> Option<&'static Job> {
        static JOB: OnceLock<Option<Job>> = OnceLock::new();
        JOB.get_or_init(|| {
            // SAFETY: chamadas Win32 com ponteiros válidos; o handle fica
            // aberto de propósito até o processo terminar.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    tracing::warn!("não foi possível criar o Job Object");
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                Some(Job(h))
            }
        })
        .as_ref()
    }

    pub fn track(process: std::os::windows::io::RawHandle) {
        if let Some(job) = job() {
            // SAFETY: `process` é o handle de um processo filho ainda aberto.
            let ok = unsafe { AssignProcessToJobObject(job.0, process as HANDLE) };
            if ok == 0 {
                tracing::warn!("processo não entrou no Job Object");
            }
        }
    }
}

/// Coloca um processo filho no job "morre junto com o Sentinel".
#[cfg(windows)]
pub fn track(child: &tokio::process::Child) {
    if let Some(h) = child.raw_handle() {
        imp::track(h);
    }
}

#[cfg(not(windows))]
pub fn track(_child: &tokio::process::Child) {}
