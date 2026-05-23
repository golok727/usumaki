use anyhow::{Context, Result};

use crate::AppConfig;
use crate::runtime::worker::{WorkerBuildOptions, create_worker};

pub fn run_headless(startup_snapshot: Option<&'static [u8]>, app_config: AppConfig) -> Result<()> {
    let main_file = &app_config.entry;
    let app_root = &app_config.app_root;
    let tokio_runtime = deno_runtime::tokio_util::create_basic_runtime();
    let main_module = deno_core::resolve_path(
        main_file
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("entry path is not valid utf-8"))?,
        app_root,
    )
    .context("failed to resolve main module path")?;

    let config = app_config.clone();

    let mut worker = {
        let _guard = tokio_runtime.enter();
        create_worker(WorkerBuildOptions {
            entry: main_file,
            app_root,
            args: config.args.clone(),
            headless: true,
            jsx_import_source: config.jsx_import_source.clone(),
            extensions: vec![crate::uzumaki::init()],
            startup_snapshot,
        })?
    };

    {
        let op_state = worker.js_runtime.op_state();
        op_state.borrow_mut().put(config);
    }

    let res = tokio_runtime.block_on(async {
        worker.execute_main_module(&main_module).await?;
        worker.run_event_loop(false).await?;
        Ok::<_, anyhow::Error>(())
    });

    if let Err(err) = res {
        print_runtime_error(&err);
    }

    Ok(())
}

fn print_runtime_error(err: &anyhow::Error) {
    eprintln!("{} {:#}", crate::terminal_colors::red_bold("Error"), err);
}
