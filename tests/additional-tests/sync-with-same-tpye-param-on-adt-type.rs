/*!
```rudra-test
test_type = "normal"
expected_analyzers = ["SendSyncVariance"]
```
!*/
struct S<'a, T>(&'a T);

impl<'a, T> Sync for S<'a, T> {}

impl<T> S<'_, T> {
    fn f<U>(&self, _: S<'_, U>) {}
}
