import os
os.environ['TF_CPP_MIN_LOG_LEVEL'] = '3'
import tensorflow as tf
from tensorflow.keras.models import Model
from tensorflow.keras.layers import Conv2D, Flatten, Dense, Input, Reshape, Add, GlobalAveragePooling2D, multiply

TTT_DIM = (3, 3, 2)
TTT_ACTION_SPACE = 9
C4_DIM = (6, 7, 2)
C4_ACTION_SPACE = 7

DIM = C4_DIM
ACTION_SPACE = C4_ACTION_SPACE

FILTERS = 128
KERNEL_SIZE = (3, 3)


def squeeze_excite_block(tensor, ratio=16):
    init = tensor
    se_shape = (1, 1, FILTERS)

    se = GlobalAveragePooling2D()(init)
    se = Reshape(se_shape)(se)
    se = Dense(FILTERS // ratio, activation='relu',
               kernel_initializer='he_normal', use_bias=False)(se)
    se = Dense(FILTERS, activation='sigmoid',
               kernel_initializer='he_normal', use_bias=False)(se)

    x = multiply([init, se])
    return x

def cnn_layer(x):
    return Conv2D(filters=FILTERS, kernel_size=KERNEL_SIZE, padding='same', strides=1,
                  activation='relu')(x)

def resnet_block(input_layer):
    x = cnn_layer(input_layer)
    x = cnn_layer(x)
    x = squeeze_excite_block(x)
    x = Add()([x, input_layer])
    return x

def get_model():
    input_layer = Input(
        shape=DIM, name="input")

    x = Conv2D(filters=FILTERS, kernel_size=KERNEL_SIZE, padding='same', strides=1,
            activation='relu', input_shape=DIM)(input_layer)

    x = resnet_block(x)
    x = resnet_block(x)
    x = resnet_block(x)
    x = resnet_block(x)
    x = resnet_block(x)

    x = resnet_block(x)
    x = resnet_block(x)
    x = resnet_block(x)
    x = resnet_block(x)
    x = resnet_block(x)

    policy_head = x
    policy_head = Conv2D(
        filters=FILTERS // 4,
        kernel_size=(1, 1),
        padding='valid',
        strides=1,
        activation='relu')(policy_head)
    policy_head = Flatten()(policy_head)

    policy_out = Dense(ACTION_SPACE, activation="softmax",
                    name="policy_head")(policy_head)

    bce = tf.keras.losses.CategoricalCrossentropy(from_logits=False)

    model = Model(inputs=input_layer, outputs=policy_out)

    model.compile(
        optimizer="SGD",
        loss=bce,
        metrics=["accuracy"]
    )

    return model