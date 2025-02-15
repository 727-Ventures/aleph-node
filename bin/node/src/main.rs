#[cfg(feature = "try-runtime")]
use aleph_node::ExecutorDispatch;
use aleph_node::{new_authority, new_full, new_partial, Cli, Subcommand};
#[cfg(feature = "try-runtime")]
use aleph_runtime::Block;
use log::warn;
use sc_cli::{clap::Parser, CliConfiguration, PruningParams, SubstrateCli};
use sc_network::config::Role;
use sc_service::PartialComponents;

const STATE_PRUNING: &str = "archive";
const BLOCKS_PRUNING: &str = "archive-canonical";

fn pruning_changed(params: &PruningParams) -> bool {
    let state_pruning_changed =
        params.state_pruning != Some(STATE_PRUNING.into()) && params.state_pruning.is_some();

    let blocks_pruning_changed =
        params.blocks_pruning != Some(BLOCKS_PRUNING.into()) && params.blocks_pruning.is_some();

    state_pruning_changed || blocks_pruning_changed
}

fn main() -> sc_cli::Result<()> {
    let mut cli = Cli::parse();
    let overwritten_pruning = pruning_changed(&cli.run.import_params.pruning_params);
    if !cli.aleph.experimental_pruning() {
        cli.run.import_params.pruning_params.state_pruning = Some(STATE_PRUNING.into());
        cli.run.import_params.pruning_params.blocks_pruning = Some(BLOCKS_PRUNING.into());
    }

    match &cli.subcommand {
        Some(Subcommand::BootstrapChain(cmd)) => cmd.run(),
        Some(Subcommand::BootstrapNode(cmd)) => cmd.run(),
        Some(Subcommand::ConvertChainspecToRaw(cmd)) => cmd.run(),
        Some(Subcommand::Key(cmd)) => cmd.run(&cli),
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, config.database), task_manager))
            })
        }
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, config.chain_spec), task_manager))
            })
        }
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    import_queue,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, import_queue), task_manager))
            })
        }
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.sync_run(|config| cmd.run(config.database))
        }
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let PartialComponents {
                    client,
                    task_manager,
                    backend,
                    ..
                } = new_partial(&config)?;
                Ok((cmd.run(client, backend, None), task_manager))
            })
        }
        #[cfg(feature = "try-runtime")]
        Some(Subcommand::TryRuntime(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            runner.async_run(|config| {
                let registry = config.prometheus_config.as_ref().map(|cfg| &cfg.registry);
                let task_manager =
                    sc_service::TaskManager::new(config.tokio_handle.clone(), registry)
                        .map_err(|e| sc_cli::Error::Service(sc_service::Error::Prometheus(e)))?;

                Ok((cmd.run::<Block, ExecutorDispatch>(config), task_manager))
            })
        }
        #[cfg(not(feature = "try-runtime"))]
        Some(Subcommand::TryRuntime) => Err("TryRuntime wasn't enabled when building the node. \
        You can enable it with `--features try-runtime`."
            .into()),
        None => {
            let runner = cli.create_runner(&cli.run)?;
            if cli.aleph.experimental_pruning() {
                warn!("Experimental_pruning was turned on. Usage of this flag can lead to misbehaviour, which can be punished. State pruning: {:?}; Blocks pruning: {:?};", 
                    cli.run.state_pruning()?.unwrap_or_default(),
                    cli.run.blocks_pruning()?,
                );
            } else if overwritten_pruning {
                warn!("Pruning not supported. Switching to keeping all block bodies and states.");
            }

            let aleph_cli_config = cli.aleph;
            runner.run_node_until_exit(|config| async move {
                match config.role {
                    Role::Authority => {
                        new_authority(config, aleph_cli_config).map_err(sc_cli::Error::Service)
                    }
                    Role::Full => {
                        new_full(config, aleph_cli_config).map_err(sc_cli::Error::Service)
                    }
                }
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use sc_service::{BlocksPruning, PruningMode};

    use super::{PruningParams, BLOCKS_PRUNING, STATE_PRUNING};

    #[test]
    fn pruning_sanity_check() {
        let state_pruning = Some(String::from(STATE_PRUNING));
        let blocks_pruning = Some(String::from(BLOCKS_PRUNING));

        let pruning_params = PruningParams {
            state_pruning,
            blocks_pruning,
        };

        assert_eq!(
            pruning_params.blocks_pruning().unwrap(),
            BlocksPruning::KeepFinalized
        );

        assert_eq!(
            pruning_params.state_pruning().unwrap().unwrap(),
            PruningMode::ArchiveAll
        );
    }
}
