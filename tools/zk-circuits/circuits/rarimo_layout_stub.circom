pragma circom 2.2.3;

// Futurenet pipeline stub: mirrors Rarimo query public signal indices used by the contract
// (birthDate at [1], nationality at [5]). NOT passport cryptography — use Rarimo query in production.
template RarimoLayoutStub() {
    signal input nullifier;
    signal input birth_date;
    signal input expiration_date;
    signal input pad3;
    signal input pad4;
    signal input nationality;
}

component main {public [nullifier, birth_date, expiration_date, pad3, pad4, nationality]} =
    RarimoLayoutStub();
