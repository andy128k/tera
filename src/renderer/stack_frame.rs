use std::collections::HashMap;

use renderer::for_loop::ForLoop;
use template::Template;
use value::{Value, ValueRef};

pub type FrameContext<'a> = HashMap<&'a str, &'a dyn Value>;

/// Enumerates the types of stack frames
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FrameType {
    /// Original frame
    Origin,
    /// New frame for macro call
    Macro,
    /// New frame for for loop
    ForLoop,
    /// Include template
    Include,
}

/// Entry in the stack frame
#[derive(Debug)]
pub struct StackFrame<'a> {
    /// Type of stack frame
    pub kind: FrameType,
    /// Frame name for context/debugging
    pub name: &'a str,
    /// Assigned value (via {% set ... %}, {% for ... %}, {% namespace::macro(a=a, b=b) %})
    ///
    /// - {% set ... %} adds to current frame_context
    /// - {% for ... %} builds frame_context before iteration
    /// - {% namespace::macro(a=a, b=b)} builds frame_context before invocation
    pub context: FrameContext<'a>,
    /// Active template for frame
    pub active_template: &'a Template,
    /// `ForLoop` if frame is for a for loop
    pub for_loop: Option<ForLoop<'a>>,
    /// Macro namespace if MacroFrame
    pub macro_namespace: Option<&'a str>,
}

impl<'a> StackFrame<'a> {
    pub fn new(kind: FrameType, name: &'a str, tpl: &'a Template) -> Self {
        StackFrame {
            kind,
            name,
            context: FrameContext::new(),
            active_template: tpl,
            for_loop: None,
            macro_namespace: None,
        }
    }

    pub fn new_for_loop(name: &'a str, tpl: &'a Template, for_loop: ForLoop<'a>) -> Self {
        StackFrame {
            kind: FrameType::ForLoop,
            name,
            context: FrameContext::new(),
            active_template: tpl,
            for_loop: Some(for_loop),
            macro_namespace: None,
        }
    }

    pub fn new_macro(
        name: &'a str,
        tpl: &'a Template,
        macro_namespace: &'a str,
        context: FrameContext<'a>,
    ) -> Self {
        StackFrame {
            kind: FrameType::Macro,
            name,
            context,
            active_template: tpl,
            for_loop: None,
            macro_namespace: Some(macro_namespace),
        }
    }

    pub fn new_include(name: &'a str, tpl: &'a Template) -> Self {
        StackFrame {
            kind: FrameType::Include,
            name,
            context: FrameContext::new(),
            active_template: tpl,
            for_loop: None,
            macro_namespace: None,
        }
    }

    /// Finds a value in the stack frame.
    /// Looks first in `frame_context`, then compares to for_loop key_name and value_name.
    pub fn find_value(self: &Self, key: &str) -> Option<ValueRef<'a>> {
        self.find_value_in_frame(key).or_else(|| self.find_value_in_for_loop(key))
    }

    /// Finds a value in `frame_context`.
    pub fn find_value_in_frame(self: &Self, key: &str) -> Option<ValueRef<'a>> {
        if let Some(dot) = key.find('.') {
            if dot < key.len() + 1 {
                if let Some(found_value) =
                    self.context.get(&key[0..dot]).map(|v| v.get_by_pointer(&key[dot + 1..]))
                {
                    return found_value.map(ValueRef::borrowed);
                }
            }
        } else if let Some(found) = self.context.get(key) {
            return Some(ValueRef::borrowed(*found));
        }

        None
    }
    /// Finds a value in the `for_loop` if there is one
    pub fn find_value_in_for_loop(&self, key: &str) -> Option<ValueRef<'a>> {
        if let Some(ref for_loop) = self.for_loop {
            // 1st case: the variable is the key of a KeyValue for loop
            if for_loop.is_key(key) {
                return Some(ValueRef::borrowed(&for_loop.get_current_key()));
            }

            let (real_key, tail) = if let Some(tail_pos) = key.find('.') {
                (&key[..tail_pos], &key[tail_pos + 1..])
            } else {
                (key, "")
            };

            // 2nd case: one of Tera loop built-in variable
            if real_key == "loop" {
                match tail {
                    "index" => {
                        return Some(ValueRef::owned(for_loop.current + 1));
                    }
                    "index0" => {
                        return Some(ValueRef::owned(for_loop.current));
                    }
                    "first" => {
                        return Some(ValueRef::owned(for_loop.current == 0));
                    }
                    "last" => {
                        return Some(ValueRef::owned(for_loop.current == for_loop.len() - 1));
                    }
                    _ => return None,
                };
            }

            // Last case: the variable is/starts with the value name of the for loop
            // The `set` case will have been taken into account before
            let v = for_loop.get_current_value();
            // Exact match to the loop value and no tail
            if key == for_loop.value_name {
                return Some(ValueRef::borrowed(v));
            }

            if real_key == for_loop.value_name && tail != "" {
                return v.get_by_pointer(tail).map(ValueRef::borrowed);
            }
        }

        None
    }

    /// Insert a value in the context
    pub fn insert(&mut self, key: &'a str, value: &'a dyn Value) {
        self.context.insert(key, value);
    }

    /// Context is cleared on each loop
    pub fn clear_context(&mut self) {
        if self.for_loop.is_some() {
            self.context.clear();
        }
    }
}
