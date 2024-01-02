# Parser-printer

This crate contains the basic building blocks for this tool.

## TODO

Things that we want to implement for sure, the question is more about when, not if

- [ ] streaming parsing/printing
- [ ] basic conflict resolution

### Done

- [x] Create lib.rs for the parser/printer stuff, and move JSON to an examples file
- [x] branching
- [x] Combinator that fails if the rest of the input is not empty (`complete` in `nom`?)
  - Strangely makes the parser fail, but the blend file is OK? It needs investigation; maybe unused data is dumped after the `ENDB` marker?
- [x] Parsing/printing blender files using the same schema description
- [x] Simple commit and checkout operations

## "Research" ideas

These ideas might not be worth the effort but could be interesting.

- [ ] A printer that consumes the input (Would this require code duplication? How much?)
- [ ] Continuation passing style to eliminate copying in the printer?
