pragma circom 2.2.3;

// DEMO-ONLY CIRCUIT - NOT PRODUCTION SOUND
// This circuit is for testing/development purposes only.
// It does NOT prove knowledge of factors of 4 as the comments claim.

template MultiplyIsFour() {
    // Very simple circuit: a * b = c
    // NOTE: This does NOT enforce c = 4 as the original comment suggested.
    // It only proves that the prover knows a and b such that c = a * b.
    signal input a;
    signal input b;
    signal output c;

    c <== a * b;
}

component main = MultiplyIsFour();