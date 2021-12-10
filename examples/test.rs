use std::time;
use hex_literal::hex;
use web3::{contract::{Contract, Options}, Transport, types::U256, Web3};
use web3::api::Eth;
use web3::contract::Error;
use web3::contract::tokens::{Detokenize, Tokenize};
use web3::transports::Http;
use web3::types::{Address, BlockNumber, FilterBuilder, Log};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let http = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(http);

    // test_hello(web3).await?;
    test_erc20(web3).await?;
    Ok(())
}

async fn test_hello(web3: Web3<Http>) -> web3::contract::Result<()> {
    // let my_account = hex!("d028d24f16a8893bd078259d413372ac01580769").into();
    let bytecode = include_str!("./res/SimpleEvent.bin");
    let accounts = web3.eth().accounts().await?;
    println!("accounts: {:?}", &accounts);
    let contract = Contract::deploy(web3.eth(), include_bytes!("./res/SimpleEvent.abi"))?
        .confirmations(0)
        .poll_interval(time::Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(bytecode, (), accounts[0])
        .await?;
    println!("contract deployed at: {}", contract.address());

    // call contract
    for _ in 0..5 {
        let tx = contract.call("hello", (), accounts[0], Options::default()).await?;
        println!("got tx: {:?}", tx);
    }

    let start_block = 0;
    let current_block = web3.eth().block_number().await.expect("");

    // Accessing existing contract
    let contract_address = contract.address();
    let existed_contract = Contract::from_json(
        web3.eth(),
        contract_address,
        include_bytes!("./res/SimpleEvent.abi"),
    )?;

    let d = events::<_ ,Address>(web3.eth(), &existed_contract, "Hello", Some(0.into()), None).await?;
    println!("{:?}", d);
    Ok(())
}

async fn test_erc20(web3: Web3<Http>) -> web3::contract::Result<()> {
    Ok(())
}

pub async fn events<T: Transport, R: Detokenize>(web3: Eth<T>, contract: &Contract<T>, event: &str, from: Option<BlockNumber>, to: Option<BlockNumber>) -> Result<Vec<(R, Log)>, Error> {
    fn to_topic<A: Tokenize>(x: A) -> ethabi::Topic<ethabi::Token> {
        let tokens = x.into_tokens();
        if tokens.is_empty() {
            ethabi::Topic::Any
        } else {
            tokens.into()
        }
    }

    let res = contract.abi().event(event).and_then(|ev| {
        let filter = ev.filter(ethabi::RawTopicFilter {
            topic0: to_topic(()),
            topic1: to_topic(()),
            topic2: to_topic(()),
        })?;
        Ok((ev.clone(), filter))
    });
    let (ev, filter) = match res {
        Ok(x) => x,
        Err(e) => return Err(e.into()),
    };

    let mut builder = FilterBuilder::default().topic_filter(filter);
    if let Some(f) = from {
        builder = builder.from_block(f);
    }
    if let Some(t)  = to {
        builder = builder.to_block(t);
    }

    let filter = builder.build();

    let logs = web3
        .logs(filter)
        .await?;
    logs.into_iter()
        .map(move |l| {
            let log = ev.parse_log(ethabi::RawLog {
                topics: l.topics.clone(),
                data: l.data.0.clone(),
            })?;

            Ok((R::from_tokens(
                log.params.into_iter().map(|x| x.value).collect::<Vec<_>>(),
            )?, l))
        })
        .collect::<Result<Vec<(R, Log)>, Error>>()
}
