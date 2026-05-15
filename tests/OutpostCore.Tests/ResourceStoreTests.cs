namespace OutpostGame.Tests;
using OutpostGame.Core.Colony;

[TestFixture]
public class ResourceStoreTests
{
    [Test]
    public void Add_And_Get_RoundTrip()
    {
        var store = new ResourceStore();
        store.Add("iron", 100f);
        Assert.That(store.Get("iron"), Is.EqualTo(100f));
    }

    [Test]
    public void Consume_DeductsCorrectly()
    {
        var store = new ResourceStore();
        store.Add("iron", 100f);
        store.TryConsume("iron", 30f);
        Assert.That(store.Get("iron"), Is.EqualTo(70f).Within(0.01f));
    }

    [Test]
    public void Consume_Fails_WhenInsufficient()
    {
        var store = new ResourceStore();
        store.Add("iron", 10f);
        Assert.That(store.TryConsume("iron", 50f), Is.False);
        Assert.That(store.Get("iron"), Is.EqualTo(10f));
    }

    [Test]
    public void Cap_ClampsFutureAdds()
    {
        var store = new ResourceStore();
        store.SetCap("iron", 50f);
        store.Add("iron", 100f);
        Assert.That(store.Get("iron"), Is.EqualTo(50f));
    }
}
