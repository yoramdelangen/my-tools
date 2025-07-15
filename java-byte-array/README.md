# Convert given input byte array representation

For a project I have sometimes certain example/output whats printed as follows:

```
[66, 192, 42, 240]
```

Output will be directly being put in the clipboard so for copying straight into
the code.

> this is a unsigned byte array representation.

I want to test this byte array in Java, to see how its being handled by my application.
Therefore it needs to be converted into java code.
Java defaults to a signed byte array representation, so the following java code is invalid:

```java
// INVALID
byte[] input = byte[]{66, 192, 42, 240}
```

this should be something like this:

```java
// INVALID
byte[] input = byte[]{66, (byte)192, 42, (byte)240}
```

## Usage

```bash
./input "[66, 192, 42, 240]"

# output should be something like:
# 66, (byte) 192, 42, (byte)240
```

> Note: there aren't any `[` `]` added.
