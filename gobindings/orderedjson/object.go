package orderedjson

type Pair struct {
	Key   string
	Value any
}

type Object struct {
	pairs map[string]Pair
	order []string
}

type InitOption func(*Object)

func WithInitialData(pairs ...Pair) InitOption {
	return func(o *Object) {
		for _, pair := range pairs {
			o.Set(pair.Key, pair.Value)
		}
	}
}

func New(opts ...InitOption) Object {
	obj := &Object{
		pairs: make(map[string]Pair),
		order: make([]string, 0),
	}
	for _, opt := range opts {
		opt(obj)
	}
	return *obj
}

// Pairs returns an iterator, enable when we upgrade to Go 1.23
// func (o *Object) Pairs() iter.Seq2[string, any] {
//	return func(yield func(string, any) bool) {
//		for _, key := range o.order {
//			pair := o.pairs[key]
//			if !yield(key, pair.Value) {
//				return
//			}
//		}
//	}
// }

func (o *Object) Keys() []string {
	return o.order
}

func (o *Object) Len() int {
	if o == nil || o.pairs == nil {
		return 0
	}
	return len(o.order)
}

func (o *Object) Contains(key string) bool {
	_, present := o.pairs[key]
	return present
}

func (o *Object) Get(key string) (any, bool) {
	pair, present := o.pairs[key]
	return pair.Value, present
}

func (o *Object) Delete(key string) {
	if !o.Contains(key) {
		return
	}
	delete(o.pairs, key)
	for i, k := range o.order {
		if k == key {
			o.order = append(o.order[:i], o.order[i+1:]...)
			break
		}
	}
}

func (o *Object) Set(key string, value any) {
	if o.Contains(key) {
		o.pairs[key] = Pair{
			Key:   key,
			Value: value,
		}
	} else {
		p := Pair{
			Key:   key,
			Value: value,
		}
		o.pairs[key] = p
		o.order = append(o.order, key)
	}
}

func (o *Object) ToMap() map[string]any {
	m := make(map[string]any)
	for _, key := range o.order {
		val := o.pairs[key].Value
		if obj, ok := val.(Object); ok {
			m[key] = obj.ToMap()
			continue
		}
		m[key] = o.pairs[key].Value
	}
	return m
}
