Now lets add support for a module and importing system (via the existing keyword `import`) that allows users to write code across multiple files. Modules can be imported as a single object that allows access to the members of the module, and users may choose to import targeted public members of a module only. A member in a module is private to the module when the name of the member starts with an `_`, otherwise it is public. Since we're adding support for modules, its important that the main function is now able to take in a filepath argument from which it loads the code to run.


Now lets add support for classes with support for multiple inheritance. Like modules, members of a class with their name starting with an `_` can only be accesed by that class and their subclasses (they're private); otherwise, class members are public.

Now lets add support for enumerations



FIX:
- test_match_nested.eth
- test_match_in_return.eth
- test_match_list.eth
- test_match_exhaustive.eth


A few test cases failed with the wrong ouputs:

1. test_match_nested.eth

```
zero
positive small
nil
nil
```

2. test_match_in_return.eth

```
newborn
toddler
nil
nil
```

3. test_match_list.eth

```
multiple elements
multiple elements
multiple elements
```

4. test_match_exhaustive.eth

```
one
nil
```

It's important to note that I've decided to rename Value::Void to Value::Nil which prints formatted string `nil`.