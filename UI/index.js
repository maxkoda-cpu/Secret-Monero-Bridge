const express = require('express');
const curl = require('curl');
const path = require('path');
const mysql = require('mysql')
const app = express();

app.use(express.static('public'))

const {
	CosmWasmClient,
	EnigmaUtils,
	Secp256k1Pen,
	SigningCosmWasmClient,
	pubkeyToAddress,
	encodeSecp256k1Pubkey
} = require("secretjs");
const SECRET_REST_URL = 'https://bootstrap.secrettestnet.io/'
const SECRET_RPC_URL = 'http://bootstrap.secrettestnet.io:26657/'
const SECRET_WS_URL = 'ws://bootstrap.secrettestnet.io:26657/websocket'
const SECRET_CHAIN_ID = 'holodeck-2'
const MNEMONIC = 'glass calm true any heart wrist client case flash option reject action'
const ADDRESS = 'secret1a25v63qp54z0u8f2zj6zxvmy6qgvy6kfkq08f4'
const port = 3118;
const doMint = async (recip, amount, txId, txKey) => {
	const bridgeMinter = 'secret13zfn3w7lystprcz5a6528pv67nnrjpq42lzlvh';
	const sxmrContract = 'secret1qqsgtlv3q9g9mg7fn42xlcr8h6ce53z7ukv3yk';
	const customFees = {
		upload: {
			amount: [{
				amount: "2000000",
				denom: "uscrt"
			}],
			gas: "2000000",
		},
		init: {
			amount: [{
				amount: "500000",
				denom: "uscrt"
			}],
			gas: "500000",
		},
		exec: {
			amount: [{
				amount: "500000",
				denom: "uscrt"
			}],
			gas: "500000",
		},
		send: {
			amount: [{
				amount: "80000",
				denom: "uscrt"
			}],
			gas: "80000",
		},
	}
	const httpUrl = "https://bootstrap.secrettestnet.io/";
	const signingPen = await Secp256k1Pen.fromMnemonic('fatigue crumble wreck rescue length admit confirm announce merge grace sketch test defy educate urban dream rude client usual bird enter finish bomb squeeze');
	const pubkey = encodeSecp256k1Pubkey(signingPen.pubkey);
	const accAddress = pubkeyToAddress(pubkey, 'secret');
	const txEncryptionSeed = EnigmaUtils.GenerateNewSeed();
	const client = new SigningCosmWasmClient(
		httpUrl,
		bridgeMinter,
		(signBytes) => signingPen.sign(signBytes),
		txEncryptionSeed, customFees
	);
	const handleMsg = {
		"mint_secret_monero": {
			"amount": amount.toString(), 
			"recipient": recip, 
			"proof": {
				"tx_id": txId, 
				"tx_key": txKey, 
				"address": recip
			}
		}
	};
	const responseContract = await client.execute("secret1yx6g7j82zp487h95t8wuutzy5uvfhfayl5dqld", handleMsg);
	console.log('Minting response: ', responseContract);
	return responseContract;
}

app.use(express.urlencoded({
	extended: false
}))
app.use(express.json())

app.get('/', (req, res) => {
	res.sendFile('./public/index.html', {
		root: __dirname
	}, (err) => {
		res.end();

		if (err) throw (err);
	});
});


app.post('/submit', (req, res) => {
	const maddress = "9yjvUbbdcFSXRKAoaCGfQQR9UkbkwVBhqYXihc1EcoPdin8YibndTrCbEgQHVNfVdcdmtbt5gFqCnBAt81QakWg83GCkibK";
	var request = require('request');
	request.post({
		headers: {
			'content-type': 'application/json'
		},
		url: 'http://127.0.0.1:18083/json_rpc',
		body: `{"jsonrpc":"2.0","id":"0","method":"check_tx_key","params":{"txid":"${req.body.txid}","tx_key":"${req.body.txkey}","address":"${maddress}"}}`
	}, function (error, response, body) {
		if (error) {
			res.send(error);
		} else {
			const resp = JSON.parse(body);
			if (!resp.error) {
				if (resp.result.confirmations && resp.result.confirmations > 0 && resp.result.received && resp.result.received > 0) {
					doMint(req.body.walletaddress, resp.result.received, req.body.txid, req.body.txkey)
						.then((r) => {
							res.send("Transaction Processed. Check your sXMR balance.<br/><br/><br/>"+ JSON.stringify(r));
						})
						.catch((err) => {
							res.send("This Monero proof of payment has already been processed.");
						});
				} else {
					res.send(`<p>Failure, could not confirm Monero transaction<br/><br/>${JSON.stringify(resp)}`);
				}
			} else {
				res.send(`<p>Failure, no Monero Confirmation`);
			}
		}
	});
})

app.get('/sxmrtoxmrsubmit', (req, res) => {
	const amount = parseInt(req.query.amount);
	const wallet = req.query.monerowallet
	console.log(amount, wallet)

	var request = require('request');
	request.post({
		headers: {
			'content-type': 'application/json'
		},
		url: 'http://127.0.0.1:18083/json_rpc',
		body: `{"jsonpc":"2.0","id":"0","method":"transfer","params":{"destinations":[{"amount":${amount},"address":"${wallet}"}],"account_index":0,"subaddr_indices":[0],"priority":0,"ring_size":11,"get_tx_key": true}}`
	}, function (error, response, body) {
		console.log(response);
		console.log(body);
		if (error) {
			res.json({success:false});
		} else {
			res.json({success:true});
		}
	});
});

app.listen(port, () => {
	console.log(`Example app listening at http://localhost:${port}`)
})
