PaymonCoin
============

PaymonCoin is the new cryptocurrency with smart-contracts and fast transactions, based on DAG. (Work In Progress)
You can download wallet (client) [here](https://github.com/PaymonTeam/paymon-wallet)

##Conversation

[`Slack channel`](https://join.slack.com/t/paymoncoin/shared_invite/enQtMzkyNjY1MTMwMzQzLTcxYzcwYjVjM2NlOTEwOGE4MjY1NjI3MzA0YjhkNTBkNWEwMzAyYTkyM2ZjYTcxYmIwYTA0NWFmMDRhNTVjMWU)

##Manual compilation

1. Install [`rustup.rs`](https://rustup.rs/).

2. Clone the source code:

   ```sh
   git clone https://github.com/PaymonTeam/paymoncoin.git
   cd paymoncoin
   ```

3. Make sure you have the right Rust compiler installed. Run

   ```sh
   rustup override set stable
   rustup update stable
   ```

4. Compile

   ```sh
   cargo build --release
   ```