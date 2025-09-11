
This is instructions for AI. The overall goal is to convert the BASIC interpreter into Rust.

The overall approach I want to take is -

1. Add a devcontainer that supports building the existing interpreter and also support the target Rust environment
2. Build the existing interpreter. Intel support is all I need.
3. Run or create relevant test on the built interpreter
4. Create Rust scaffolding for the interpreter
5. Chop up the work in 5 to 10 batches
6. For each batch do step 7 to 9
7. Create Rust tests
8. Run Rust tests to ensure all tests fails
9. Implement the code and ensure the tests succeeds
10. Run all test to ensure the Rust interpreter works
