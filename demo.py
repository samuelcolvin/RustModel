import timeit
from rustmodel import SchemaValidator
from dataclasses import dataclass
from pydantic import BaseModel


class RustModel:
    __slots__ = ('__pydantic_model_data__',)
    # __pydantic_model_data__: ModelData

    def __getattr__(self, item):
        return self.__pydantic_model_data__.get_attr(item)

    def model_dump(self):
        return self.__pydantic_model_data__.model_dump()

    def model_dump_json(self):
        return self.__pydantic_model_data__.model_dump_json()


class MyModel(RustModel):
    foo: str
    bar: int
    spam: str
    ham: int
    egg: str = 'y'


validator = SchemaValidator(
    {
        'type': 'model',
        'cls': MyModel,
        'fields': [
            {
                'name': 'foo',
                'schema': {'type': 'string'},
            },
            {
                'name': 'bar',
                'schema': {'type': 'int'},
                'default': 123,
            },
            {
                'name': 'spam',
                'schema': {'type': 'string'},
                'default': 'x',
            },
            {
                'name': 'ham',
                'schema': {'type': 'int'},
                'default': 456,
            },
            {
                'name': 'egg',
                'schema': {'type': 'string'},
                'default': 'y',
            }
        ]
    }
)

input_data = {
    'ham': 123,
    'foo': 'hello',
    # 'egg': 'EGG',
    'bar': 456,
    'spam': 'SPAM',
}
model: MyModel = validator.validate_python(input_data)
print('foo:', model.foo)
print('bar:', model.bar)
print('model dump:', model.model_dump())
print('model dump json:', model.model_dump_json())

timer = timeit.Timer("v.validate_python(input_data)", globals={'v': validator, 'input_data': input_data})
n, t = timer.autorange()
iter_time = t / n
print(f'RustModel: {iter_time * 1_000_000_000:0.2f} ns')


@dataclass
class MyDataclass:
    foo: str
    bar: int
    spam: str
    ham: int
    egg: str = 'y'


timer = timeit.Timer("MyDataclass(**input_data)", globals={'MyDataclass': MyDataclass, 'input_data': input_data})
n, t = timer.autorange()
iter_time = t / n
print(f'Dataclass: {iter_time * 1_000_000_000:0.2f} ns')


class MyPydanticModel(BaseModel):
    foo: str
    bar: int
    spam: str
    ham: int
    egg: str = 'y'


timer = timeit.Timer(
    "MyPydanticModel.model_validate(input_data)",
    globals={'MyPydanticModel': MyPydanticModel, 'input_data': input_data}
)
n, t = timer.autorange()
iter_time = t / n
print(f'Pydantic: {iter_time * 1_000_000_000:0.2f} ns')
