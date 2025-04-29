defmodule FlowerTest do
  use ExUnit.Case
  doctest Flower

  test "greets the world" do
    assert Flower.hello() == :world
  end
end
