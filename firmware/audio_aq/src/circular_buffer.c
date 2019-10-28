/*
 * Circular Buffer implementation
 */

#include "circular_buffer.h"

/*
 * Retorna o espaço disponível
 * do buffer circular
 */
int RingBufferSpaceAvailable(RingBuffer* ringb)
{
	return ((ringb->start - ringb->end + ringb->size - 1) % ringb->size);
}

/*
 * Retorna o espaço ocupado no
 * buffer circular
 */
int RingBufferSpaceUsed(RingBuffer* ringb)
{
	return ((ringb->end - ringb->start + ringb->size) % ringb->size);
}

/*
 * Inicializa a estrutura do
 * buffer circular
 */
void RingBufferInit(RingBuffer* ringb, EndPointBuffer *buffer, uint16_t bsize)
{
	ringb->end = 0;
	ringb->start = 0;
	ringb->buffer = buffer;
	ringb->size = bsize;
}

/*
 * Escreve um byte no buffer
 */
int RingBufferWrite(RingBuffer* ringb, EndPointBuffer *data)
{
	uint32_t NextIndex = (ringb->end + 1) % ringb->size;
	if (NextIndex != ringb->start)
	{
		ringb->buffer[ringb->end] = *data;
		ringb->end = NextIndex;
		return 0;
	}
	else
	{
		return -1;
	}
}

/*
 * Lê um byte do buffer
 */
int RingBufferRead(RingBuffer* ringb, EndPointBuffer *data)
{
	uint32_t NextIndex = (ringb->start + 1) % ringb->size;
	if (ringb->start != ringb->end)
	{
		*data = ringb->buffer[ringb->start];
		ringb->start = NextIndex;
		return 0;
	}
	else
	{
		return -1;
	}
}
