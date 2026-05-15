pragma circom 2.2.3;

// DEMO-ONLY CIRCUIT
// This circuit demonstrates basic multiplication in Circom.
// It proves that the prover knows values a and b such that c = a * b.
// This is useful for testing the proof generation pipeline
// but does NOT prove anything about specific values like factors of 4.

template Multiply() {
    signal input a;
    signal input b;
    signal output c;

    // Basic constraint: c must equal a * b
    c <== a * b;
}

component main = Multiply();