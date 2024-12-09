use std::{env, time::Duration};
use subxt_signer::sr25519::dev;
use zombienet_sdk::NetworkConfigBuilder;
use subxt::{ OnlineClient, SubstrateConfig};

#[subxt::subxt(runtime_metadata_path = "metadata-files/rococo.scale")]
pub mod rococo {}

#[subxt::subxt(runtime_metadata_path = "metadata-files/coretime-rococo.scale")]
mod coretime_rococo {}

use coretime_rococo::
	runtime_types::{
		pallet_broker::types::ConfigRecord as BrokerConfigRecord,
		sp_arithmetic::per_things::Perbill,
	};
type CoretimeRuntimeCall = coretime_rococo::runtime_types::coretime_rococo_runtime::RuntimeCall;
type CoretimeUtilityCall = coretime_rococo::runtime_types::pallet_utility::pallet::Call;
type CoretimeBrokerCall = coretime_rococo::runtime_types::pallet_broker::pallet::Call;

// Relaychain nodes

const ALICE: &str = "alice";
const BOB: &str = "bob";
const CHARLIE: &str = "charlie";
const DAVE: &str = "dave";
const BEST_BLOCK_METRIC: &str = "block_height{status=\"best\"}";

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let network_builder = NetworkConfigBuilder::new().with_relaychain(|r| {
        r.with_chain("rococo-local")
            .with_default_command("polkadot")
            .with_node(|node| {
                node.with_name(ALICE)
                .with_initial_balance(100_000_000_000_000_000_000)
            })
            .with_node(|node| node.with_name(BOB))
            .with_node(|node| node.with_name(CHARLIE))
            .with_node(|node| node.with_name(DAVE))
    })
    .with_parachain(|p|{
        p.with_id(1005)
        .with_chain("coretime-rococo-local")
        .with_default_command("polkadot-parachain")
        .with_collator(|c|{
            c.with_name("coretime-collator")
        })
    });

    let config = network_builder.build().map_err(|errs| {
        let e = errs
            .iter()
            .fold("".to_string(), |memo, err| format!("{memo} \n {err}"));
        anyhow::anyhow!(e)
    }).expect("Config should be valid. qed");

    // force native provider
    env::set_var("ZOMBIE_PROVIDER", "native");
    let spawn_fn = zombienet_sdk::environment::get_spawn_fn();
    let network = spawn_fn(config).await.expect("Spawn should work. qed");


    println!("running 🚀🚀🚀");

    // set broker (based on https://github.com/paritytech/polkadot-sdk/blob/master/polkadot/zombienet_tests/smoke/0004-configure-broker.js)

    // wait 10 blocks of coretime chain
    let coretime = network.get_node("coretime-collator").expect("coretime-collator should exist. qed");
    let _ = coretime.wait_metric(BEST_BLOCK_METRIC, |x| x > 10_f64).await;


    let coretime_client: OnlineClient<SubstrateConfig> = coretime.wait_client().await.expect("coretime-collator should be accesible through ws. qed");

    // Alice is sudo in coretime-rococo-local
    let sudo = dev::alice();

	// Initialize broker and start sales

	coretime_client
		.tx()
		.sign_and_submit_default(
			&coretime_rococo::tx().sudo().sudo(CoretimeRuntimeCall::Utility(
				CoretimeUtilityCall::batch {
					calls: vec![
						CoretimeRuntimeCall::Broker(CoretimeBrokerCall::configure {
							config: BrokerConfigRecord {
								advance_notice: 5,
								interlude_length: 1,
								leadin_length: 1,
								region_length: 1,
								ideal_bulk_proportion: Perbill(100),
								limit_cores_offered: None,
								renewal_bump: Perbill(10),
								contribution_timeout: 5,
							},
						}),
						CoretimeRuntimeCall::Broker(CoretimeBrokerCall::set_lease {
							task: 1005,
							until: 1000,
						}),
						CoretimeRuntimeCall::Broker(CoretimeBrokerCall::start_sales {
							end_price: 45_000_000,
							extra_cores: 100,
						}),
					],
				},
			)),
			&sudo,
		)
		.await.expect("Coretime initialization should work. qed");


    loop {
        tokio::time::sleep(Duration::from_secs(60)).await;
    }

}