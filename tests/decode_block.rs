use antelope::chain::checksum::Checksum256;
use antelope::util::hex_to_bytes;
use telos_translator_rs::block::ProcessingEVMBlock;
use telos_translator_rs::types::ship_types::{ShipResult, SignedBlock};

#[test]
fn decode_block() {
    let encoded_hex = "013f87ac1414ac873fe9fbc89fae05b52a12b01fb0fa09215172e120c15a42640969808307f085ac1414ac85f0ea1f4c41f5a4df5ec8351b468da3bf036b4570f1294bc3bcd0eccf280100a3e11111e1a300a403fad6ab3d40d086a073edc5149e34b64a03ed084267b69876348501ffa2e11111e1a2ff0aacb4241d323c25acdfcbf1b1dcc6faf7ca4000bee11a4283b798d90198045482335980b1915e5d268dcaf00011e1a2ff0aacb4241d323c25acdfcbf1b1dcc6faf7ca4000bee11a4283b798d9efea968311df53f076bb44d0ac1c9f0cd07c9a10a516160446f3a013dd5c9c26454a56fcb481414f817364f0f1a20de5edb7baf552692923322f2af746ca5e042c14000000000020606995341eee2eec7ef12c79cbc295340ff7e29c5087be3afdb789ece394b7e7641b1db39ef242725ab02eb4b2c04e78a0e65619dbb6c1b896f422bed7c6569a0200960000000f0101002035150c1f6f855dc2395e48f117a104ec7fc97ce9447a16140de1295e9142bac34adb88651c7a89dd8b1bc45fe2c9622fe8e37385551d4a8381aa81279b67233f00004ae5040765eba15409223900000000019091b97952a4a6930000000000a0a69301e0afc646dd5cbbe10000000000a0a69318e0afc646dd5cbbe155d40a000000000004430c0000000000000064000000150101001f0ab76ef0326e68b0a3c4f94964435d81eb8ea4e7e71cdc5581f3ac5a57872b5c6a083db892698279c0fe9e885d82e3cef940b31141d3a7030b3ebb86bfcc952200007cc7040765afa1cfd0faa50000000001003c8580d1d8b0690000008c4688683c01003c8580d1d8b06900000000a8ed32324a083236333538353638403031393233333238366666333736646431386335336139353635356636643666343563636232346663343730353032313063363632306537666564643531346400000181080300b9949b2232e25a5edf29557e607687c1e909b3461ff6c93fb60ddd0de462248d0064000000005c000000000000000000000000000000000101010001000000000000ea3055c7ed7aed82179726fc11bc006affb7bbbccc218aa627a6581ff894da6264476921962953020000001ca83c1200000000010000000000ea305527621a12000000000a060000000000ea30550000000000ea305500000000221acfa4010000000000ea305500000000a8ed323274538233598021a2f160aaa241000011e1a2fe43221c0c0417db8344fa365b507669fddac705e5384c6d8ad758c9a25484a1bcd870afb5cc205a5075542c31ab041e03828d3839094a22ee59dba7623f31b15bdb52bfb8c3cee10c6eb443b51c2222eb7a938c8cfcea9986d2c6f95a2c140000000000440000000000000000010000000000ea305500000000000000000000000000000001000000000000000000000000000000000000c193811ed6b9f1197bf4df2732dcb198c3f5d13ff0471fd7bc80170cb47a7b9f00960000000f5f000000000000007800000000000000000101010001009091b97952a4a69351d21ccfd369bf6c3ce8d0a90b8a643a314d00aec78e30753bfbe30546565fd52296295302000000a0df84000000000001e0afc646dd5cbbe13b0d1d000000000018049091b97952a4a6939091b97952a4a6930000000000a0a69301e0afc646dd5cbbe10000000000a0a69318e0afc646dd5cbbe155d40a000000000004430c000000000000220000000000000011200a20312e646966663a3330303030303000000000000000000100e5040765eba1540922390000000001002035150c1f6f855dc2395e48f117a104ec7fc97ce9447a16140de1295e9142bac34adb88651c7a89dd8b1bc45fe2c9622fe8e37385551d4a8381aa81279b67233f0000fcf3fb7a4b98dfc90ae32c8cbfa31ed722e7ac5eb8902299655d7ba8d19999140064000000153000000000000000a80000000000000000010101000100003c8580d1d8b069e428350c81eab4495be9bc7fdacddc4b771739febeba57278f889ee602166b5623962953020000001ab518000000000001003c8580d1d8b0691cb51800000000000101003c8580d1d8b069003c8580d1d8b0690000008c4688683c01003c8580d1d8b06900000000a8ed32324a0832363335383536384030313932333332383666663337366464313863353361393536353566366436663435636362323466633437303530323130633636323065376665646435313464000f000000000000000000000000000000000100c7040765afa1cfd0faa50000000001001f0ab76ef0326e68b0a3c4f94964435d81eb8ea4e7e71cdc5581f3ac5a57872b5c6a083db892698279c0fe9e885d82e3cef940b31141d3a7030b3ebb86bfcc95220001960d03000c636f6e74726163745f726f770b0142009091b97952a4a6939091b97952a4a6930000000000954dc60000000000954dc69091b97952a4a69318b80b000000000000e50984000000000080feea568e050600013a000000000000ea30550000000000ea3055000000406573bda9000000406573bda90000000000ea305510d2090000000000008093dc1400000000014a000000000000ea30550000000000ea30550000c093ba6c32bd0000c093ba6c32bd0000000000ea305520806336d3e5a9d83580d3cdd81c88683c1100000029000000a39433592343325901b402000000000000ea30550000000000ea305570b3922aeaa41ac270b3922aeaa41ac20000000000ea305589028021a2f160aaa241000000001510a23934324ca3ca00000000305543d9414ca3ca00000000909d74716a4ca3ca0000000010dd37f750773155000000008021a2f160aaa2410000000080b1915e5d268dca0c000000406f85f1603a9d810c00000080683c3ebbe9d6740c00000060b0c634aaac32dd0c000000000090265d95b1340c00000080d3cdd81c88683c0c000000e0a79157314ca3ca0c00000000000020397a403d0c000000100c7277999635c50c0000001092bbc9484ca3ca00000000108c3be61a4ca3ca0000000050cf54ea324ca3ca00000000806954791a87afaa00000000500fa651651f9d49000000008063f68c46aaa2ca00000000500f75eaaa563155000000000142000000000000ea30550000000000ea3055000000804473686400000080447368640000000000ea305518eac7f23e06fba83f50c3000000000000409c000000000000013a000000000000ea30550000000000ea3055000000604473686400000060447368640000000000ea305510a0cea32661050600c887527f5603b242013d000000000000ea30550000000000ea3055000000404473686400000040447368640000000000ea3055130000e575335953823359ea1574a4cc322c440001e801000000000000ea30550000000000ea3055000000004473686400000000447368640000000000ea3055bd010000100000000000e8030000c02709000c000000f40100001400000064000000400d0300f4010000f049020064000000100e00005802000080533b00ffff07000a00060080be4c7c08000000ed2afdcd020000008ddf2159010000002e82335923433259a040a64d8e05060000000000000000003a00000000000000d703000064cb1373c5000000a0a18ba4607d05001500621415d47811b942e3243259dca2e1111c81335900000000400032469e560000000000000000000000000001af02000000000000ea30550000000000ea30550000c057219de8ad8021a2f160aaa2418021a2f160aaa24184028021a2f160aaa241f19316cd0b9d6a42000261096f8f70563e1e3b0fda936a94b957caa60b209b214dca6690b7847051c9e801001168747470733a2f2f63616c656f732e696f18000000c830cf0000000000ff340000a040a64d8e050600ac0d010000006950726f6475636572206163636f756e74207761732064656163746976617465642062656361757365206974207265616368656420746865206d6178696d756d206d697373656420626c6f636b7320696e207468697320726f746174696f6e2074696d656672616d652e0000000002000000b9d3254e000100000001000261096f8f70563e1e3b0fda936a94b957caa60b209b214dca6690b7847051c9e801000037000000000000ea305500000000000000000000a06b3a88683cf5a2e111000000000000000000ea30550d00f5a2e111201297568e0506000137000000000000ea305500000000000000000000a06b3a88683cffa2e111000000000000000000ea30550d00ffa2e111605de3568e050600000e7265736f757263655f757361676503013b00003c8580d1d8b0690054823359abe6f9090000000050010000000000000054823359751f4b0600000000ce0000000000000028d8050000000000013b00e0afc646dd5cbbe10054823359549a3a00000000007c00000000000000005482335931613700000000009a000000000000003610000000000000013b000000000000ea305500548233590000000000000000000000000000000000548233590106f40500000000c800000000000000ff0551010000000000157265736f757263655f6c696d6974735f7374617465010153000000a3e1117777f20c00000000f7010000000000000000a3e1114e51951b000000002a03000000000000eccc66040a0000002fbe0afc0b000000d56d4e29030000000000803e0000000000c2eb0b00000000";
    let encoded_bytes = hex_to_bytes(encoded_hex);

    let mut decoder = antelope::chain::Decoder::new(encoded_bytes.as_slice());
    let ship_result = &mut ShipResult::default();
    decoder.unpack(ship_result);

    match ship_result {
        ShipResult::GetStatusResultV0(_) => panic!("Should not be GetStatusResultV0"),
        ShipResult::GetBlocksResultV0(r) => {
            if let Some(b) = &r.this_block {
                println!("Got block: {}", b.block_num);
                let mut block = ProcessingEVMBlock::new(
                    1,
                    b.block_num,
                    Checksum256::default(),
                    b.block_num,
                    Checksum256::default(),
                    r.clone(),
                );
                block.deserialize();
            } else {
                panic!("GetBlocksResultV0 without a block");
            }
        }
    }
}

#[test]
fn decode_ebr_signed_block() {
    let encoded_hex = "e71d5a470000000000ea30550000000f426521e456687e7012ec37220dc6835c8fcd74e919386ca32dac74d5303542cb90721d0226e10ec09fafb83c8390685199481e39a42cd12845e8814a62ec8d0ccaef2a21ffcccdfb417c08bb2c40bc83a7b6af4615cc3c1bd4319b3a958700000000010100000015104208d7b7aa7e100003a21294cf229605ef75ba4c75456a827b40dc611a843a37d6a5c91ad38b421635d055ae67d2eb983b0002dd8202fc4e1efe58f3fe54191497c96cab7fcd3c4f9c22ac7367b4d354020dcc8021a2f160aaa241000261096f8f70563e1e3b0fda936a94b957caa60b209b214dca6690b7847051c9e860268d0a5d73305500025183a9a93702bf4071149220310289ececa7ac41147f1140baa32e5d96a281adc0299f2e1e9d3055000234626a8ab821e6ac3d2f7db5846f07ae66e57e861d8a3f550ff0f3c605bdc913901dbd5925ea30550003254fadc5dc48c3269b61362103da76aa5ff3902f3425983a0f93f34943660492500f75d16425315500027a29a1c9896e3bcb73b2515190953c96b3559f1a065eaaae938ea18175b7ec0f80683c3ebbe9d67400027c7388090f588e77fb29426b9b7e484cdeae7d385ad1f48af8a196ff00a0270e406f85f1603a9d8100022dbbfafe230112ebc2754e84649f743806f65beba09e94c6dfd51799664b47ee80695479526632a20002a2ad00b124ad4abf28ac33c4fd523a6262ee62bd8a9f4079360ab2b475d55262a0129dc8244ca3ca0003f08f1745a004f01bf387b23fe75c879b9b1a5ffc7c81dd16f5c24caf5d0d320710a23934324ca3ca000299f03526e26c2aab82c15494192b39c1a9e0e79a787d42dcc031648d6bf805f950cf54ea324ca3ca000316df5be18b4ab25f94ce96196a704b67211a852b10f5b04991b0d27b061a11551092bbc9484ca3ca0003c6061290f1a8e30a3468596f3bd01c006d0a42bbeeca6ad33b287559e6c20a3b508f93c6494ca3ca000311739eac79593cbb4b66f8589a849e2a0a8f8a7f341491fd767267ca4691c8d6002ff55c4d4ca3ca00038dd0ce1c4e11588403fcff7af0d4dc3610271c3dcbb0dad61dde43ced95ca4298055a2136a4ca3ca0003c94e8c3b04696ee217b84f5b97ad3070c70a7727da5854e85c0f16880cf7dd6c7015339e6e4ca3ca000378828b10d7860ec05a1cf47470713702713ca742e83da8b9e5e9524ebb4dfbdd70d5d6144db371cc0002302d29c8a3aa078dd02f27eadc806decb0b15f83526eeac2b4cd1afe5da5e65860a2d25f4db571cc0003daeded528da9013a399dd59929be3196549b643bf48576c67513380901698e9f80a96a28eba432dd00030459df9a25dc52e1340f82eb5a7eefc41e6e6b4636f0fd12c487ce47c8fbc78300001f5e2c27cc0240062a0131ea43e1d950ccc8b70b9d9b5930225551f7693bf0ec710b90a46abe1a0aa98f9fa1bcb907eaa125e75462bdee3079458905961e87fa11020094090000170101001f4f187016bdcb22f5d96f0e61788ebec97e0628be1a4adeda58222f1dfedce1b76d753957fa6f311fdfd9609ef5e893e91af0bbe221bf9f4ee7dd8ef4f481943b0000628f521a5c614207d5ba2f00000000010000000000ea3055c08fca86a9a8d2d401a09867fd4896876200000000a8ed323230a09867fd48968762a09867fd4896876268e77c060000000004544c4f5300000028c87c060000000004544c4f5300000000006e09000017010100206d519b49c5cd1e6e7bad69bc1edc37ec7ac4e20677d42b8c35b81d1f4b07cbaf5439263bae4675dffc6cb3d74cab87008c36508376061a1ab948ef261809cda10000628f521a5c614207d5ba2f00000000010000000000ea3055c08fca86a9a8d2d401a09863f6499bbe6900000000a8ed323230a09863f6499bbe69a09863f6499bbe69d0ef80000000000004544c4f53000000d0ef80000000000004544c4f530000000000";
    let encoded_bytes = hex_to_bytes(encoded_hex);

    let mut decoder = antelope::chain::Decoder::new(encoded_bytes.as_slice());
    let block = &mut SignedBlock::default();
    decoder.unpack(block);
}