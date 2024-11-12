<div align="center">

# Monty
#### "MCTS is cool."

</div>

## Compiling
To compile, run
```
make
```
The required networks will be downloaded automatically (and validated).

## Development

Development of Monty is facilitated by [montytest](https://tests.montychess.org/tests).
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

Monty is free and distributed under the [**GNU Affero General Public License**][license-link] (AGPL v3). Essentially,
this means you are free to do almost exactly what you want with the program, including distributing it among your friends, 
making it available for download from your website, selling it (either by itself or as part of some bigger software package), 
or using it as the starting point for a software project of your own.

The only real limitation is that whenever you distribute Monty in some way, including distribution over a network (such as providing 
access to Monty via a web application or service), you MUST always include the license and the full source code (or a pointer to where 
the source code can be found) to generate the exact binary you are distributing. If you make any changes to the source code, these 
changes must also be made available under AGPL v3.

[license-link]:       https://github.com/official-monty/Monty/blob/master/Copying.txt
