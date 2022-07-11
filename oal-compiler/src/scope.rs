use crate::errors::Result;
use crate::module::ModuleSet;
use crate::node::NodeRef;
use crate::scan::Scan;
use oal_syntax::ast::AsExpr;
use oal_syntax::atom::Ident;
use std::collections::HashMap;

pub type Scope<T> = HashMap<Ident, T>;

pub struct Env<'a, T> {
    scopes: Vec<Scope<T>>,
    modules: Option<&'a ModuleSet<T>>,
}

impl<'a, T> Env<'a, T>
where
    T: AsExpr,
{
    pub fn new(mods: Option<&'a ModuleSet<T>>) -> Env<'a, T> {
        Env {
            scopes: vec![Scope::new()],
            modules: mods,
        }
    }

    #[cfg(test)]
    pub fn head(&self) -> &Scope<T> {
        self.scopes.last().unwrap()
    }

    pub fn declare(&mut self, n: Ident, e: T) {
        self.scopes.last_mut().unwrap().insert(n, e);
    }

    pub fn lookup(&self, n: &Ident) -> Option<&T> {
        self.scopes
            .iter()
            .rev()
            .map(|s| s.get(n))
            .skip_while(Option::is_none)
            .map(|s| s.unwrap())
            .next()
    }

    #[cfg(test)]
    pub fn exists(&self, n: &Ident) -> bool {
        self.scopes.last().unwrap().contains_key(n)
    }

    pub fn within<F, R>(&mut self, mut f: F) -> R
    where
        F: FnMut(&mut Self) -> R,
    {
        self.open();
        let r = f(self);
        self.close();
        r
    }

    pub fn import(&mut self, path: &str) -> Result<()> {
        if let Some(mods) = self.modules {
            let loc = mods.base.join(path)?;
            if let Some(m) = mods.get(&loc) {
                m.scan(self, &mut Env::new(None), &mut declaration_scan)
            } else {
                // All modules that are to be imported must be present in the module-set.
                panic!("unknown module: {}", loc)
            }
        } else {
            Ok(())
        }
    }

    fn open(&mut self) {
        self.scopes.push(Scope::new());
    }
    fn close(&mut self) {
        self.scopes.pop();
    }
}

fn declaration_scan<T>(acc: &mut Env<T>, _env: &mut Env<T>, node: NodeRef<T>) -> Result<()>
where
    T: AsExpr,
{
    if let NodeRef::Decl(decl) = node {
        acc.declare(decl.name.clone(), decl.expr.clone())
    }
    Ok(())
}
