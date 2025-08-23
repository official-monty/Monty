EXE = monty

# Use EXE suffix for Windows
ifeq ($(OS),Windows_NT)
    EXEEXT := .exe
else
    EXEEXT :=
endif

NAME := $(EXE)$(EXEEXT)

# Check for Git Bash / MSYS by inspecting shell
UNAME_S := $(shell uname -s)
ifeq ($(findstring MINGW,$(UNAME_S)),MINGW)
    IS_MINGW := 1
endif

# Pick proper env var setting syntax
ifeq ($(OS),Windows_NT)
  ifeq ($(IS_MINGW),1)
    INVOKE := RUSTFLAGS="-Ctarget-cpu=native" cargo +stable rustc --release
  else
    INVOKE := cmd /C "set RUSTFLAGS=-Ctarget-cpu=native && cargo +stable rustc --release"
  endif
else
  INVOKE := RUSTFLAGS="-Ctarget-cpu=native" cargo +stable rustc --release
endif

LINK := -- --emit link=$(NAME)

default:
	$(INVOKE) --bin monty --features=embed $(LINK)

raw:
	$(INVOKE) --bin monty --features=embed,raw $(LINK)

montytest:
	$(INVOKE) --bin monty --features=uci-minimal,tunable $(LINK)

noembed:
	$(INVOKE) --bin monty $(LINK)

gen-value:
	$(INVOKE) --package datagen --bin datagen --features value $(LINK)

gen-policy:
	$(INVOKE) --package datagen --bin datagen --features policy $(LINK)
