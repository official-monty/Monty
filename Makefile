SHELL := /bin/bash

EXE = monty

ifeq ($(OS),Windows_NT)
	NAME := $(EXE).exe
	OLD := monty-$(VER).exe
	AVX2 := monty-$(VER)-avx2.exe
else
	NAME := $(EXE)
	OLD := monty-$(VER)
	AVX2 := monty-$(VER)-avx2
endif

default:
	cargo rustc --release --bin monty --features=embed -- -C target-cpu=native --emit link=$(NAME)

# Ensure latest stable rust is installed
.PHONY: update-stable
update-stable:
	source $$HOME/.cargo/env && rustup update stable

montytest: update-stable
	cargo +stable rustc --release --bin monty --features=uci-minimal,tunable -- -C target-cpu=native --emit link=$(NAME)

noembed:
	cargo rustc --release --bin monty -- -C target-cpu=native --emit link=$(NAME)

gen:
	cargo rustc --release --package datagen --bin datagen -- -C target-cpu=native --emit link=$(NAME)

release:
	cargo rustc --release --bin monty --features=embed -- --emit link=$(OLD)
	cargo rustc --release --bin monty --features=embed -- -C target-cpu=x86-64-v2 -C target-feature=+avx2 --emit link=$(AVX2)