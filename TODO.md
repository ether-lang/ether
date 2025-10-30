Now lets add support for a module and importing system (via the existing keyword `import`) that allows users to write code across multiple files. Modules can be imported as a single object that allows access to the members of the module, and users may choose to import targeted public members of a module only. A member in a module is private to the module when the name of the member starts with an `_`, otherwise it is public. Since we're adding support for modules, its important that the main function is now able to take in a filepath argument from which it loads the code to run.


Now lets add support for classes with support for multiple inheritance. Like modules, members of a class with their name starting with an `_` can only be accesed by that class and their subclasses (they're private); otherwise, class members are public.

Now lets add support for enumerations



FIX:
- test_recursion.eth


A test case failed with the wrong ouputs:

1. test_recursion.eth

```
Factorial of 5:
120
Factorial of 0:
1
Fibonacci sequence:
0
1
0
-3
-8
-15
-24
-35
```
