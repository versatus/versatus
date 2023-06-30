(module
  (type $increment_t (func (param i32) (result i32)))
  (func $increment_f (type $increment_t) (param $value i32) (result i32)
    local.get $value
    i32.const 1
    i32.add)
  (export "increment" (func $increment_f)))
