using System;
using Hyperlight.Wrapper;
using Xunit;

namespace Hyperlight.Tests;
public class StringWrapperTest
{
    private const string ORIG_STR = "StringWrapperTest EXAMPLE";
    [Fact]
    public void Test_Create_String()
    {
        using var ctx = new Context("Test_Create_String");
        using var str = StringWrapper.FromString(ctx, ORIG_STR);
        Assert.True(str.HandleWrapper.IsString());
        Assert.Equal(ORIG_STR, str.HandleWrapper.GetString());
        Assert.Equal(
            $"StringWrapper: {ORIG_STR}",
            str.ToString()
        );
    }
}
