#![no_std]

extern crate alloc;

use core::any::TypeId;

struct EmptyTypeId;

pub struct MapTypeIdIterator<'a, M> {
    map: &'a M,
    depth: usize,
}

impl<'a, M: Map> Iterator for MapTypeIdIterator<'a, M> {
    type Item = TypeId;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.map.type_id(self.depth);
        if id == TypeId::of::<EmptyTypeId>() {
            return None;
        }
        self.depth += 1;
        Some(id)
    }
}

impl<'a, M: Map> MapTypeIdIterator<'a, M> {
    fn new(map: &'a M) -> Self {
        Self { depth: 0, map }
    }
}

#[derive(Clone, Debug)]
pub struct StackedMap;

pub trait Map: Sized {
    type Inner;
    fn into_inner(self) -> Self::Inner;
    fn clear(self) -> StackedMap {
        StackedMap
    }
    fn contains<T: 'static>(&self) -> bool {
        self.get::<T>().is_some()
    }
    fn get<T: 'static>(&self) -> Option<&T>;
    fn get_mut<T: 'static>(&mut self) -> Option<&mut T>;
    fn insert<T: 'static>(self, value: T) -> InsertedMap<Self, T>;
    fn remove<T: 'static>(self) -> Removed<Self, T>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn type_id(&self, depth: usize) -> TypeId;
    fn type_id_iter<'a>(&'a self) -> MapTypeIdIterator<'a, Self>;
}

impl Map for StackedMap {
    type Inner = StackedMap;

    fn into_inner(self) -> Self::Inner {
        StackedMap
    }

    fn get<T: 'static>(&self) -> Option<&T> {
        None
    }

    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        None
    }

    fn insert<T: 'static>(self, value: T) -> InsertedMap<Self, T> {
        InsertedMap::Inserted(self, value)
    }

    fn remove<T: 'static>(self) -> Removed<Self, T> {
        Removed::NotFound(self)
    }

    fn len(&self) -> usize {
        0
    }

    fn type_id(&self, _depth: usize) -> TypeId {
        TypeId::of::<EmptyTypeId>()
    }

    fn type_id_iter<'a>(&'a self) -> MapTypeIdIterator<'a, Self> {
        MapTypeIdIterator::new(self)
    }
}

impl<M: Map, U: 'static> Map for InsertedMap<M, U> {
    type Inner = M;

    fn into_inner(self) -> Self::Inner {
        match self {
            InsertedMap::Existed { map, .. }
            | InsertedMap::Inserted(map, _)
            | InsertedMap::None(map) => map,
        }
    }

    fn get<T: 'static>(&self) -> Option<&T> {
        match self {
            InsertedMap::Existed { map, .. } | InsertedMap::None(map) => map.get::<T>(),
            InsertedMap::Inserted(map, value) => {
                if TypeId::of::<T>() == TypeId::of::<U>() {
                    return Some(unsafe { core::mem::transmute(value) });
                }
                map.get::<T>()
            }
        }
    }

    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            InsertedMap::Existed { map, .. } | InsertedMap::None(map) => map.get_mut::<T>(),
            InsertedMap::Inserted(map, value) => {
                if TypeId::of::<T>() == TypeId::of::<U>() {
                    return Some(unsafe { core::mem::transmute(value) });
                }
                map.get_mut::<T>()
            }
        }
    }

    fn insert<T: 'static>(mut self, value: T) -> InsertedMap<Self, T> {
        match &mut self {
            InsertedMap::Existed { map, .. } | InsertedMap::None(map) => {
                if let Some(v) = map.get_mut::<T>() {
                    let old = core::mem::replace(v, value);
                    InsertedMap::Existed { map: self, old }
                } else {
                    InsertedMap::Inserted(self, value)
                }
            }
            InsertedMap::Inserted(map, v) => {
                if TypeId::of::<T>() == TypeId::of::<U>() {
                    let value_transmuted = unsafe { core::mem::transmute_copy(&value) };
                    core::mem::forget(value);
                    let old = core::mem::replace(v, value_transmuted);
                    let old_transmuted = unsafe { core::mem::transmute_copy(&old) };
                    core::mem::forget(old);
                    InsertedMap::Existed {
                        map: self,
                        old: old_transmuted,
                    }
                } else {
                    if let Some(v) = map.get_mut::<T>() {
                        let old = core::mem::replace(v, value);
                        InsertedMap::Existed { map: self, old }
                    } else {
                        InsertedMap::Inserted(self, value)
                    }
                }
            }
        }
    }

    fn remove<T: 'static>(self) -> Removed<Self, T> {
        match self {
            InsertedMap::Existed { map, old } => match map.remove::<T>() {
                Removed::Removed { map, value } => Removed::Removed {
                    map: InsertedMap::Existed { map, old },
                    value,
                },
                Removed::NotFound(map) => Removed::NotFound(InsertedMap::Existed { map, old }),
            },
            InsertedMap::Inserted(map, value) => {
                if TypeId::of::<T>() == TypeId::of::<U>() {
                    let v = unsafe { core::mem::transmute_copy(&value) };
                    core::mem::forget(value);
                    return Removed::Removed {
                        map: InsertedMap::None(map),
                        value: v,
                    };
                }
                Removed::NotFound(InsertedMap::Inserted(map, value))
            }
            InsertedMap::None(map) => match map.remove::<T>() {
                Removed::Removed { map, value } => Removed::Removed {
                    map: InsertedMap::None(map),
                    value,
                },
                Removed::NotFound(map) => Removed::NotFound(InsertedMap::None(map)),
            },
        }
    }

    fn len(&self) -> usize {
        match self {
            InsertedMap::Existed { map, .. } | InsertedMap::None(map) => map.len(),
            InsertedMap::Inserted(map, _) => map.len() + 1,
        }
    }

    fn type_id(&self, depth: usize) -> TypeId {
        if depth == 0 {
            match self {
                InsertedMap::Existed { .. } | InsertedMap::None(_) => TypeId::of::<EmptyTypeId>(),
                InsertedMap::Inserted(_, _) => TypeId::of::<U>(),
            }
        } else {
            match self {
                InsertedMap::Existed { map, .. }
                | InsertedMap::Inserted(map, _)
                | InsertedMap::None(map) => map.type_id(depth - 1),
            }
        }
    }

    fn type_id_iter<'a>(&'a self) -> MapTypeIdIterator<'a, Self> {
        MapTypeIdIterator::new(self)
    }
}

#[derive(Clone, Debug)]
pub enum Removed<M, T> {
    Removed { map: M, value: T },
    NotFound(M),
}

impl<M: Map, U: 'static> Map for Removed<M, U> {
    type Inner = M;

    fn into_inner(self) -> Self::Inner {
        match self {
            Removed::Removed { map, .. } | Removed::NotFound(map) => map,
        }
    }

    fn get<T: 'static>(&self) -> Option<&T> {
        match self {
            Removed::Removed { map, .. } | Removed::NotFound(map) => map.get(),
        }
    }

    fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        match self {
            Removed::Removed { map, .. } | Removed::NotFound(map) => map.get_mut(),
        }
    }

    fn insert<T: 'static>(self, value: T) -> InsertedMap<Self, T> {
        match self {
            Removed::Removed { map, value: v } => match map.insert(value) {
                InsertedMap::Existed { map, old } => InsertedMap::Existed {
                    map: Removed::Removed { map, value: v },
                    old,
                },
                InsertedMap::Inserted(map, value) => {
                    InsertedMap::Inserted(Removed::Removed { map, value: v }, value)
                }
                InsertedMap::None(map) => InsertedMap::None(Removed::Removed { map, value: v }),
            },
            Removed::NotFound(map) => match map.insert(value) {
                InsertedMap::Existed { map, old } => InsertedMap::Existed {
                    map: Removed::NotFound(map),
                    old,
                },
                InsertedMap::Inserted(map, value) => {
                    InsertedMap::Inserted(Removed::NotFound(map), value)
                }
                InsertedMap::None(map) => InsertedMap::None(Removed::NotFound(map)),
            },
        }
    }

    fn remove<T: 'static>(self) -> Removed<Self, T> {
        match self {
            Removed::Removed { map, value: v } => match map.remove() {
                Removed::Removed { map, value } => Removed::Removed {
                    map: Removed::Removed { map, value: v },
                    value,
                },
                Removed::NotFound(map) => Removed::NotFound(Removed::Removed { map, value: v }),
            },
            Removed::NotFound(map) => match map.remove() {
                Removed::Removed { map, value } => Removed::Removed {
                    map: Removed::NotFound(map),
                    value,
                },
                Removed::NotFound(map) => Removed::NotFound(Removed::NotFound(map)),
            },
        }
    }

    fn len(&self) -> usize {
        match self {
            Removed::Removed { map, .. } | Removed::NotFound(map) => map.len(),
        }
    }

    fn type_id(&self, depth: usize) -> TypeId {
        match self {
            Removed::Removed { map, .. } | Removed::NotFound(map) => map.type_id(depth),
        }
    }

    fn type_id_iter<'a>(&'a self) -> MapTypeIdIterator<'a, Self> {
        MapTypeIdIterator::new(self)
    }
}

#[derive(Clone, Debug)]
pub enum InsertedMap<M, T> {
    Existed { map: M, old: T },
    Inserted(M, T),
    None(M),
}

#[cfg(test)]
mod tests {
    use alloc::string::String;
    use alloc::vec;
    use alloc::vec::Vec;
    use super::*;

    #[test]
    fn map() {
        let map = StackedMap;
        assert_eq!(map.len(), 0);
        let map = map.insert(1).insert(2).insert(3);
        assert_eq!(map.get::<String>(), None);
        assert_eq!(map.get::<i32>(), Some(&3));
        assert_eq!(map.len(), 1);
        let map2 = map.clone();
        let map3 = map.clone();
        assert!(matches!(map.remove::<String>(), Removed::NotFound(_)));
        assert!(matches!(
            map2.remove::<i32>(),
            Removed::Removed { map: _, value: 3 }
        ));
        let map = map3.insert(());
        let map = map.insert("hi");
        assert_eq!(
            map.type_id_iter().collect::<Vec<_>>(),
            vec![TypeId::of::<&'static str>(), TypeId::of::<()>()]
        );
    }
}
