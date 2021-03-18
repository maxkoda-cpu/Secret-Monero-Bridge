This folder contains the Proof-of-Concept (PoC) for the Secret Monero Bridge.
The PoC is a prototype provided as a simple reference model for the project's further development.

To keep things simple, the PoC doesn't implement multi-signature. The actual project deliverables will use multi-signature for security purposes.

The PoC will use the Monero and Secret Network testnets for transfer of value.

The PoC will illustrate two stories:

1. Converting XMR to sXMR
2. Converting sXMR to XMR

Brief descriptions for these stories follow:

**XMR -> sXMR:**

To convert XMR to sXMR the user will send an amount of XMR to the Secret Monero Bridge's Monero wallet. 
The user will then use the Secret Monero Bridge web application to provide the Monero proof-of-payment, along with the Secret wallet address
where sXMR will be deposited. The user will then click the submit button provided on the web page.

After the user supplies the requested information and clicks the submit button, the application will verify the Monero proof-of-payment, and once verified,
transfer the corresponding sXMR to the provided Secret wallet address. The XMR transferred to the Secret Monero Bridge's Monero wallet remains locked until 
sXMR tokens are moved back onto the Monero blockchain. Proof-of-Swap receipts will be persisted for a period of time on the Secret Monero Bridge.

*In the PoC, the user will be providing the Monero Proof-of-Payment in the UI. The actual project will provide Monero wallet functionality within the Secret Monero Bridge UI. This will allow users to directly transfer XMR to the Secret Monero Bridge Monero wallet and automatically collect, then verify the Monero Proof-of-Payment. Users will not have to manually provide the Monero Proof-of-Payment thereby improving overall the user experience.*

**sXMR -> XMR:**

To convert sXMR tokens to XMR, the user will interact with the Secret Monero Bridge web application. The user will indicate the amount of sXMR to convert and provide the Monero wallet address that will receive the XMR. The user will then click the submit button and the sXMR tokens will be transfered to the Secret Monero Bridge (taken out of circulation), and the XMR transferred to the user provided Monero address. A Monero proof-of-payment receipt will be provided to the user. Proof-of-Swap receipts will be persisted for a period of time on the Secret Monero Bridge.

**Monero Proof-of-Payment and the Secret Monero Bridge Proof-of-Swap:**

https://github.com/maxkoda-cpu/Secret-Monero-Bridge/blob/main/Docs/Proof-of-Swap.pdf

Users can provide mathematical proof that they made a Monero payment. In this project that proof is called the "Monero Proof-of-Payment".
The Secret Monero Bridge builds on the Monero Proof-of-Payment to construct a Proof-of-Swap which links a Monero Proof-of-Payment and a Secret Network transaction hash which represents a transfer of the sXMR secret token. The Secret Monero Bridge Proof-of-Swap is classified as private data and will be persisted in an encrypted state within the Secret Monero Bridge. Monero Proof-of-Payment proves the transfer of XMR to a destination Monero wallet address. The Secret Monero Bridge Proof-of-Swap proves the transfer of sXMR in relation to a linked Monero Proof-of-Payment. The Secret Monero Bridge intends to use Proof-of-Swap receipts as evidence in any dispute proceedings.

**Monero Testnet:**

A monerod (Monero daemon) running on the Monero testnet, a Monero wallet, and the monero-wallet-rpc are being established for the Secret Monero Bridge web application. The web application will interface with the monero-wallet-rpc to verify Monero proof-of-payments (required in XMR->sXMR swaps) as well as for sending XMR to user Monero wallets (in sXMR->XMR swaps).



