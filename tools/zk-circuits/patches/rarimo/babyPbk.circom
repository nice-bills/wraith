pragma circom 2.1.6;

// Public key from Baby Jubjub private key (matches circomlib BabyPbk interface).
include "curve.circom";

template BabyPbk() {
    signal input in;
    signal output Ax;
    signal output Ay;

    component mul = BabyjubjubBase8Multiplication();
    mul.scalar <== in;
    Ax <== mul.out[0];
    Ay <== mul.out[1];
}
