**This is preliminary information subject to change...**

In order to take a more holistic approach to building a decentralized system, we will be considering deploying the Secret Monero Bridge web application
as a Decentralized application that resides on the Interplanetary File System (IPFS). This design would deliver a web application that is **not** hosted
via a web server, but rather on IPFS, downloaded from IPFS and exectued locally

The design pattern we are considering would provide an immutable hash that would provide a vector to launch the web application on a local machine. While the 
web application would be available via an immutable hash, the web application content would be mutable. This is accomplished via a unique bootstrap mechanism 
packaged within the immutable hash (web application) that makes a call to a secret contract on the Secret Network, to get an additional IPFS hash to render the web application content. 

So the link to the web application would be immutable, but the web application calls a secret contract to obtain another IPFS link to render the web
application content. Versioning of the web application can be made by publishing subsequent versions to IPFS and then updating the web application content link
in the secret contract.

This package design has security features that simplify verification of the authenticity of the web application. Since all content on IPFS is immutable
(you change the content, you change the hash), user's simply need to verify the web applications IPFS hash, once verified, users can rest assured that the
application is legitimate and has **not** been tampered with.

This package design is also censorship resistant, and could be classified as an unstoppable Dapp.

Stay tuned for additional information on this track of development.

