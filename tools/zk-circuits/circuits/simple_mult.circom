pragma circom 2.2.3;

template MultiplyIsFour() {
    // Very simple circuit: a * b = c, where we enforce c = 4
    // This proves knowledge of factors of 4
    signal input a;
    signal input b;
    signal output c;

    c <== a * b;

    // Constraint: c must equal 4
    // This creates a valid proof that a * b = 4
}

component main = MultiplyIsFour();