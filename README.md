Google Sheets Crypto Server
===========================

Provides Google Sheets with Crypto price and wallet balance.

* Provides crypto price with data from Coingecko.
* Provides wallet balance query from various blockchains.

Usage
-----

Host the server somewhere (e.g. GCP, AWS).

Get Coin/Token Price:
```
=IMPORTXML("https://YOUR_SERVER_HOST/quote?ids=bitcion,ethereum,anchorust", "//Quotes/price/text()")
```

Get Wallet Balance:
```
=IMPORTXML("https://YOUR_SERVER_HOST/wallet_balance?chain_id=<CHAIN_ID>&address=<YOUR_ADDRESS>", "//WalletBalance/text()")
```

Supported Chain ID:
* ethereum
* polygon
* avalanche
* bsc
* polkadot
* terra

Get Wallet ERC20 Token Balance:
```
=IMPORTXML("https://YOUR_SERVER_HOST/wallet_balance?chain_id=<CHAIN_ID>&token=<ERC20_CONTRACT_ADDRESS>&address=<YOUR_ADDRESS>", "//WalletBalance/text()")
```

Additional Notes for Terra
--------------------------

To get terra native coin balance, set `token=COIN`, current `COIN` supports:

* LUNA
* UST

e.g.
```
=IMPORTXML("https://YOUR_SERVER_HOST/wallet_balance?chain_id=terra&token=LUNA&address=terra107q76k5uu3atgwz695vdcfee5qz9ukyz3jj0cs", "//WalletBalance/text()")
```
