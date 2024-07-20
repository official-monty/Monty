<div align="center">

# Monty
#### "MCTS is cool."

[![GitHub release (latest by date)](https://img.shields.io/github/v/release/official-monty/Monty?style=for-the-badge)](https://github.com/official-monty/Monty/releases/latest)
[![Commits](https://img.shields.io/github/commits-since/official-monty/Monty/latest?style=for-the-badge)](https://github.com/official-monty/Monty/commits/main)

</div>

## Compiling
To compile without embedding the networks, run
```
make EXE=<output path>
```
when running the executable it will search for the networks in the current working directory.

To compile and embed the networks in the exectuable, run
```
make embed EXE=<output path> EVALFILE=<value net path> POLICYFILE=<policy net path>
```

## Development

Development of Monty is facilitated by [montytest](https://montychess.org/tests).
If you want to contribute, it is recommended to look in:
- [src/mcts/helpers.rs](src/mcts/helpers.rs) - location of functions that
calculate many important search heuristics, e.g. CPUCT scaling
- [src/mcts.rs](src/mcts.rs) - the actual search logic

Functional patches are required to pass on montytest, with an STC followed by an LTC test.

## ELO History

<div align="center">

| Version | Release Date | CCRL 40/15 | CCRL Blitz | CCRL FRC |
| :-: | :-: | :-: | :-: | :-: |
| [1.0.0](https://github.com/jw1912/monty/releases/tag/v1.0.0) | 28th May 2024 | - | 3076 | 3107 |
| [0.1.0](https://github.com/jw1912/monty/releases/tag/v0.1.0) | 26th March 2024 | - | - | 2974 |

</div>

## Terms of use

Monty is free and distributed under the [**GNU General Public License version 3**][license-link] (GPL v3). Essentially,
this means you are free to do almost exactly what you want with the program, including distributing it among your friends, 
making it available for download from your website, selling it (either by itself or as part of some bigger software package), 
or using it as the starting point for a software project of your own.

The only real limitation is that whenever you distribute Monty in some way, you MUST always include the license and the full 
source code (or a pointer to where the source code can be found) to generate the exact binary you are distributing. If you make 
any changes to the source code, these changes must also be made available under GPL v3.

[license-link]:       https://github.com/official-monty/Monty/blob/master/Copying.txt
