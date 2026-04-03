#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqLineKind {
    Generic,
    Timer,
    Serial,
    TlbShootdown,
}

#[derive(Debug, Clone, Copy)]
pub struct IrqLineDescriptor<Id> {
    pub id: Id,
    pub kind: IrqLineKind,
    pub label: &'static str,
}

impl<Id: Copy> IrqLineDescriptor<Id> {
    pub const fn new(id: Id, kind: IrqLineKind, label: &'static str) -> Self {
        Self { id, kind, label }
    }
}

#[inline(always)]
pub fn classify_irq_line<Id: Copy + Eq>(
    id: Id,
    table: &[IrqLineDescriptor<Id>],
    default_label: &'static str,
) -> IrqLineDescriptor<Id> {
    for descriptor in table {
        if descriptor.id == id {
            return *descriptor;
        }
    }

    IrqLineDescriptor::new(id, IrqLineKind::Generic, default_label)
}
