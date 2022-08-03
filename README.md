# Substrate node with quadratic voting pallet

This project attempts to create a pallet which can be used to introduce quadratic voting to a substrain chain.
It allows voting by reserving fees on the voter. The fees increase quadratically with each vote on a single topic by the same user.

## Voting Pallet

The voting pallet allows voting for groups of proposals (topics) in each era. Each era === 10 blocks.
During an era, it's possible to vote for current active proposals which were proposed in the previous era.
It's also possible to create new proposals which will become available by the next era.

Available actions:

- submit_topic:
    Here a signed user can submit a topic which is stored alongside it's hash.
    A fee is required, to submit a proposal.

-- vote_topic:
    Allows you cote for a hash in the current era. A fee is charged for each vote,
    and is a function of the square of the number of votes you have for that topic multiplied by the default weight.

- get_current_topics:
    Here a user can get all topics hashes which are available to be voted in the current era.

- get_next_topics:
    Get the topics which are queued to be voted in the next era

- get_topic_preimage:
    Get the details of a topic given it's hash

- get_era_winners:
    Returns a map of the era number, to the hash that won in that era


## Running
There's no UI attached, but you can start up the backend with the following command

```
cargo build --release && ./target/release/node-template --dev
```

And then you can manually interact with the pallet via RPC calls and runtime calls.

## Running tests
An alternative way to explore the codebase is via the tests. 

```
cargo test  -- --show-output

```
Will run the tests, through creating a topic, and voting for topics.
