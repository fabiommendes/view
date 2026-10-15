"""Exemplo de módulo Python com classe, função e docstring."""

from dataclasses import dataclass


@dataclass
class Invoice:
    """Uma fatura com itens em centavos."""

    number: str
    items: list[int]

    def total(self) -> int:
        # Soma os itens; o valor vem em centavos para evitar float.
        return sum(self.items)


def format_brl(cents: int) -> str:
    reais, centavos = divmod(cents, 100)
    return f"R$ {reais},{centavos:02d}"


if __name__ == "__main__":
    inv = Invoice(number="2026-001", items=[1990, 500, 12345])
    print(f'Fatura {inv.number}: {format_brl(inv.total())}')
    print("Pronto.")
