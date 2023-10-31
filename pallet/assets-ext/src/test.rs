
[test]
fn rpc_asset_balance() {
	let initial_balance = 5_000_000;
	let alice = 1 as MockAccountId;
	let xrp_asset_id = 2 as AssetId;

	test_ext()
		.with_balances(&[(alice, initial_balance)])
		.with_asset(xrp_asset_id, "XRP", &[(alice, initial_balance)])
		.build()
		.execute_with(|| {
			assert_eq!(AssetsExt::reducible_balance(xrp_asset_id, &alice, false), initial_balance);
		});
}
