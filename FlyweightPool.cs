using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace toucca
{
    public class FlyweightPool<T>
    {
        public FlyweightPool(Func<T> factory)
            {
                this.factory = factory;
            }
        private readonly ConcurrentBag<T> values = new();
        private readonly Func<T> factory;

        public T Get()
        {
            return values.TryTake(out T value) ? value : factory();
        }

        public void Return(T value)
        {
            values.Add(value);
        }
    }
}
