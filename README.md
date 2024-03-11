# Solana Timelock

![License LGPLv3](https://img.shields.io/badge/License-LGPLv3-violet.svg)

A simple program for delayed transaction execution on Solana. Unaudited; use
at your own risk.

## Why use a timelock?

Basically, a timelock can provide extra security and/or decentralization. Benefits include:

- if a deployer key is compromised, users can move their funds out of the protocol before the attacker can upgrade the program
- users can verify that new changes to the program or to protocol parameters (e.g., fees) are acceptable
- projects can claim some level of decentralization even when a central team (i.e., x Labs) has the ability to update the program and/or protocol parameters

Timelocks were introduced by [Compound Finance](https://medium.com/compound-finance/compound-governance-5531f524cf68)
in 2020, and have been well-adopted by the Ethereum DeFi ecosystem.

## How to use

Example usage is demonstrated in [tests/solana_timelock.ts](./tests/solana_timelock.ts).

## Deployment address

The program is currently live on devnet at
[tiME1hz9F5C5ZecbvE5z6Msjy8PKfTqo1UuRYXfndKF](https://explorer.solana.com/address/tiME1hz9F5C5ZecbvE5z6Msjy8PKfTqo1UuRYXfndKF?cluster=devnet).
You may verify that the deployed program matches the source by using the
`anchor verify` command.
