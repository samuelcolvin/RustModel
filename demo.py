import json
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
            },
            {
                'name': 'spam',
                'schema': {'type': 'string'},
            },
            {
                'name': 'ham',
                'schema': {'type': 'int'},
            },
            {
                'name': 'egg',
                'schema': {'type': 'string'},
                'default': 'omelette',
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
input_json = json.dumps(input_data).encode()
# model: MyModel = validator.validate_python(input_data)
model: MyModel = validator.validate_json(input_json)
print('foo:', model.foo)
print('bar:', model.bar)
print('model dump:', model.model_dump())
print('model dump json:', model.model_dump_json())

justify = 20
timer = timeit.Timer("v.validate_python(input_data)", globals={'v': validator, 'input_data': input_data})
n, t = timer.autorange()
iter_time = t / n
print('RustModel python:'.ljust(justify), f'{iter_time * 1_000_000_000:0.2f} ns')

timer = timeit.Timer("v.validate_json(input_json)", globals={'v': validator, 'input_json': input_json})
n, t = timer.autorange()
iter_time = t / n
print('RustModel JSON:'.ljust(justify), f'{iter_time * 1_000_000_000:0.2f} ns')


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
print('Dataclass python:'.ljust(justify), f'{iter_time * 1_000_000_000:0.2f} ns')

timer = timeit.Timer(
    "MyDataclass(**json_loads(input_json))",
    globals={'MyDataclass': MyDataclass, 'json_loads': json.loads, 'input_json': input_json}
)
n, t = timer.autorange()
iter_time = t / n
print('Dataclass JSON:'.ljust(justify), f'{iter_time * 1_000_000_000:0.2f} ns')


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
print('Pydantic python:'.ljust(justify), f'{iter_time * 1_000_000_000:0.2f} ns')
timer = timeit.Timer(
    "MyPydanticModel.model_validate_json(input_json)",
    globals={'MyPydanticModel': MyPydanticModel, 'input_json': input_json}
)
n, t = timer.autorange()
iter_time = t / n
print('Pydantic JSON:'.ljust(justify), f'{iter_time * 1_000_000_000:0.2f} ns')
