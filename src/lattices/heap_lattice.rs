use crate::lattices::ConstLattice;
use crate::lattices::state_lattice::VariableState;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeapValue {
    HeapBase,
    Bounded4GB,
    LucetTables,
    GuestTable0,
    GlobalsBase,
}

pub type HeapValueLattice = ConstLattice<HeapValue>;

pub type HeapLattice = VariableState<HeapValueLattice>;

#[test]
fn heap_lattice_test() {
    use crate::lattices::reaching_defs_lattice::LocIdx;
    use crate::lattices::Semilattice;

    let x1 = HeapValueLattice { v: None };
    let x2 = HeapValueLattice {
        v: Some(HeapValue::HeapBase),
    };
    let x3 = HeapValueLattice {
        v: Some(HeapValue::HeapBase),
    };
    let x4 = HeapValueLattice {
        v: Some(HeapValue::Bounded4GB),
    };

    assert_eq!(x1 == x2, false);
    assert_eq!(x2 == x3, true);
    assert_eq!(x3 == x4, false);

    assert_eq!(x1 != x2, true);
    assert_eq!(x2 != x3, false);
    assert_eq!(x3 != x4, true);

    assert_eq!(x1 > x2, false);
    assert_eq!(x2 > x3, false);
    assert_eq!(x3 > x4, false);

    assert_eq!(x1 < x2, true);
    assert_eq!(x2 < x3, false);
    assert_eq!(x3 < x4, false);

    assert_eq!(
        x1.meet(&x2, &LocIdx { addr: 0, idx: 0 }) == HeapValueLattice { v: None },
        true
    );
    assert_eq!(
        x2.meet(&x3, &LocIdx { addr: 0, idx: 0 })
            == HeapValueLattice {
                v: Some(HeapValue::HeapBase)
            },
        true
    );
    assert_eq!(
        x3.meet(&x4, &LocIdx { addr: 0, idx: 0 }) == HeapValueLattice { v: None },
        true
    );
}
