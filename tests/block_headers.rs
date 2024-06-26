use alloy::primitives::FixedBytes;
use antelope::{api::client::{APIClient, DefaultProvider}, chain::{checksum::Checksum256, name::Name, signature::Signature, Encoder, Packer}};
use telos_translator_rs::{block::Block, types::{ship_types::{BlockHeader, BlockPosition, GetBlocksResultV0, GetStatusResultV0, ShipResult, SignedBlock, SignedBlockHeader}, types::NameToAddressCache}};


#[tokio::test]
async fn genesis_mainnet() {

    let http_endpoint = "https://mainnet.telos.net".to_string();

    let api_client: APIClient<DefaultProvider> = APIClient::<DefaultProvider>::default_provider(http_endpoint.clone()).expect("Failed to create API client");
    let native_to_evm_cache = NameToAddressCache::new(api_client);

    let zero_bytes = FixedBytes::from_slice(
        &vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

    let evm_chain_id_mainnet = 40;
    let chain_id_mainnet = Checksum256::from_hex("4667b205c6838ef70ff7988f6e8257e8be0e1284a2f59699054a018f743b1d11").unwrap();

    let genesis_num = 36;
    let genesis_id = Checksum256::from_hex("00000024796a9998ec49fb788de51614c57276dc6151bd2328305dba5d018897").unwrap();
    let genesis_pos = BlockPosition { block_num: genesis_num, block_id: genesis_id };

    let genesis_signed_block = SignedBlock {
        header: SignedBlockHeader {
            header: BlockHeader {
                timestamp: 1544636786,
                producer: Name::new("eosio"),
                confirmed: 0,
                previous: Checksum256::from_hex("000000232f2e3dfee35a0d63ddc3aac8209c91976da50af0031017fef802aefc").unwrap(),
                transaction_mroot: Checksum256::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap(),
                action_mroot: Checksum256::from_hex("551892564885f8b77fde73ad92f5d29be8607e6773f3704ce3e77d065cab24e4").unwrap(),
                schedule_version: 0,
                new_producers: None,
                header_extensions: vec![],
            },
            producer_signature: Signature::from_string("SIG_K1_K2DTFe8jakB7cyrVMnZJecT3QugC54wapCdpPHhro5eWsCNyBCFZEEsRr59GB9RHDttj52dgxaEuyQM7R1TJcAR7Yv8ZAc").unwrap()
        },
        transactions: vec![],
        block_extensions: vec![]
    };

    let genesis_block_bytes = Encoder::pack(&genesis_signed_block);

    let first_block_num = 37;
    let first_block_id = Checksum256::from_hex("00000025452bb767b3e6543d4513b84cabef53ddfaf64f85170fb5dd2fa148b5").unwrap();
    let first_pos = BlockPosition { block_num: first_block_num, block_id: first_block_id };

    let mut block = Block::new(
        evm_chain_id_mainnet,
        genesis_num as u64,
        genesis_id,
        GetBlocksResultV0 {
            head: first_pos.clone(),
            last_irreversible: first_pos.clone(),
            this_block: Some(genesis_pos.clone()),
            prev_block: None,
            block: Some(genesis_block_bytes),
            traces: Some(vec![]),
            deltas: Some(vec![]),
        },
    );

    block.deserialize();

    let evm_genesis = block.generate_evm_data(
        zero_bytes.clone(),
        &native_to_evm_cache
    ).await;

    println!("genesis: {:#?}", evm_genesis);
    println!("hash: {:#?}", evm_genesis.hash_slow());
}
