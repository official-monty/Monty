<div align="center">

# Monty
#### "MCTS is cool."

</div>

## Compiling
To compile, run `make`. The required networks will be downloaded automatically (and validated).
This requires `make` and a recent enough rust version (see the [MSRV](Cargo.toml)) installed via `rustup` (the official way).

## Analysis

Monty is the state-of-the-art engine for CPU-based contempt analysis. Its contempt parameter is calibrated to represent the effective rating difference 
between us and the opponent, and it has been validated across a wide range from –600 Elo (us being weaker) to +600 Elo (us being stronger).

When using contempt in analysis, it is crucial the Contempt_Analysis flag is set to true. Below is an example in the En Croissant GUI:

<img src="https://github.com/user-attachments/assets/a07371a8-5815-4237-a102-ac6aaaa4ca54" width="400">


## Development & Project Structure

#### Testing

Development of Monty is facilitated by [montytest](https://tests.montychess.org/tests).
Functional patches are required to pass on montytest, with an STC followed by an LTC test.
Nonfunctional patches may be required to pass non-regression test(s) if there are any concerns.

#### Source Code

The main engine code is found in [src/](src/), containing all the search code and network inference code.

There are a number of other crates found in [crates/](crates/):
- [`montyformat`](crates/montyformat/)
    - Core chess implementation
    - Policy/value data formats
    - All other crates depend on this
- [`datagen`](crates/datagen/)
    - Intended to be ran on montytest, there is no need to run it locally (unless testing changes)
- [`train-value`](crates/train-value/)
    - Uses [bullet](https://github.com/jw1912/bullet)
- [`train-policy`](crates/train-policy/)
    - Uses [bullet](https://github.com/jw1912/bullet) & extends it with custom operations

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
